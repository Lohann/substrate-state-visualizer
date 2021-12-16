// #![cfg_attr(not(feature = "std"), no_std)]

mod utils;

use hash_db::{HashDB, Hasher, EMPTY_PREFIX};
use hex;
use sp_core::H256;
use trie_db::node::NodeHandlePlan;
use trie_db::{node::NodeKey, NodeCodec as NodeCodecT, Trie, TrieMut};
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub struct Blake2Hasher;

impl Hasher for Blake2Hasher {
    type Out = sp_core::hash::H256;
    type StdHasher = hash256_std_hasher::Hash256StdHasher;
    const LENGTH: usize = 32;

    fn hash(x: &[u8]) -> Self::Out {
        sp_core::hashing::blake2_256(x).into()
    }
}

type Layout = sp_trie::Layout<Blake2Hasher>;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub enum TrieNode {
    Empty {
        id: Option<H256>,
    },
    Leaf {
        id: Option<H256>,
        nibbles: NodeKey,
        value: Vec<u8>,
    },
    Branch {
        id: Option<H256>,
        value: Option<Vec<u8>>,
        children: Box<[Option<TrieNode>; 16]>,
    },
    NibbledBranch {
        id: Option<H256>,
        nibbles: NodeKey,
        value: Option<Vec<u8>>,
        children: Box<[Option<TrieNode>; 16]>,
    },
}

fn u8_to_hex_vec(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::<u8>::with_capacity(bytes.len() * 2);
    unsafe {
        output.set_len(output.capacity());
    }
    hex::encode_to_slice(bytes, &mut output).unwrap();
    output
}

fn u8_to_hex(bytes: &[u8]) -> String {
    let output = u8_to_hex_vec(bytes);
    String::from_utf8(output).unwrap()
}

impl TrieNode {
    fn encode_children(children: &Box<[Option<TrieNode>; 16]>) -> js_sys::Array {
        let array: js_sys::Array = Default::default();
        for (i, maybe_child) in children.iter().enumerate() {
            if let Some(child) = maybe_child {
                let child_node = child.encode();
                let char_code: [u8; 1] = if i < 10 {
                    [(i as u8) + 48] // 0-9
                } else {
                    [(i as u8) + 87] // a-f
                };
                let output = String::from_utf8(char_code.to_vec()).unwrap();
                let nibbles = js_sys::JsString::from(output);
                let key = js_sys::JsString::from("parent_nibble");
                js_sys::Reflect::set(&child_node, &key, &nibbles).unwrap();
                array.push(&child_node);
            }
        }
        array
    }

    pub fn encode(&self) -> js_sys::Object {
        let node: js_sys::Object = Default::default();
        let node_type: js_sys::JsString;
        let maybe_node_id: Option<H256>;
        let maybe_node_nibbles: Option<NodeKey>;
        let maybe_node_value: Option<Vec<u8>>;
        let maybe_node_children: Option<js_sys::Array>;
        match self {
            TrieNode::Leaf { id, nibbles, value } => {
                node_type = js_sys::JsString::from("Leaf");
                maybe_node_id = id.clone();
                maybe_node_nibbles = Some(nibbles.clone());
                maybe_node_value = Some(value.to_vec());
                maybe_node_children = None;
            }
            TrieNode::Branch {
                id,
                value,
                children,
            } => {
                node_type = js_sys::JsString::from("Branch");
                maybe_node_id = id.clone();
                maybe_node_nibbles = None;
                maybe_node_value = value.to_owned().map(|v| v.to_vec()).clone();
                maybe_node_children = Some(Self::encode_children(children));
            }
            TrieNode::NibbledBranch {
                id,
                nibbles,
                value,
                children,
            } => {
                node_type = js_sys::JsString::from("Branch");
                maybe_node_id = id.clone();
                maybe_node_nibbles = Some(nibbles.clone());
                maybe_node_value = value.to_owned().map(|v| v.to_vec());
                maybe_node_children = Some(Self::encode_children(children));
            }
            TrieNode::Empty { id } => {
                node_type = js_sys::JsString::from("Empty");
                maybe_node_id = id.clone();
                maybe_node_nibbles = None;
                maybe_node_value = None;
                maybe_node_children = None;
            }
        }

        let k0 = js_sys::JsString::from("type");
        js_sys::Reflect::set(&node, &k0, &node_type).unwrap();

        if let Some(node_id) = maybe_node_id {
            let key = js_sys::JsString::from("id");
            let node_id = js_sys::JsString::from(u8_to_hex(&node_id.0));
            js_sys::Reflect::set(&node, &key, &node_id).unwrap();
        } else {
            let key = js_sys::JsString::from("id");
            js_sys::Reflect::set(&node, &key, &JsValue::NULL).unwrap();
        }
        if let Some(nibbles) = maybe_node_nibbles {
            let key = js_sys::JsString::from("nibbles");
            let mut output = u8_to_hex_vec(&nibbles.1);
            if nibbles.0 > 0 {
                output.drain(0..1);
            }
            let output = String::from_utf8(output).unwrap();
            let nibbles = js_sys::JsString::from(output);
            js_sys::Reflect::set(&node, &key, &nibbles).unwrap();
        }
        if let Some(value) = maybe_node_value {
            let key = js_sys::JsString::from("value");
            let output: js_sys::Uint8Array = value.as_slice().into();
            js_sys::Reflect::set(&node, &key, &output).unwrap();
        }
        if let Some(value) = maybe_node_children {
            let key = js_sys::JsString::from("children");
            js_sys::Reflect::set(&node, &key, &value).unwrap();
        }
        node
    }
}

fn decode_children_recursive(
    bytes: &[u8],
    children: [Option<NodeHandlePlan>; 16],
    db: &sp_trie::MemoryDB<Blake2Hasher>,
) -> Box<[Option<TrieNode>; 16]> {
    let mut children_array: Box<[Option<TrieNode>; 16]> = Default::default();
    for (index, maybe_child) in children.iter().enumerate() {
        if let Some(child) = maybe_child {
            match child {
                NodeHandlePlan::Hash(range) => {
                    let mut key: H256 = Default::default();
                    key.0.copy_from_slice(&bytes[range.start..range.end]);
                    if let Some(value) = db.get(&key, EMPTY_PREFIX) {
                        let node = decode_recursive(value.as_slice(), Some(key), db);
                        children_array[index] = Some(node);
                    }
                }
                NodeHandlePlan::Inline(range) => {
                    let node = decode_recursive(&bytes[range.start..range.end], None, db);
                    children_array[index] = Some(node);
                }
            }
        }
    }
    children_array
}

fn decode_recursive(
    bytes: &[u8],
    node_id: Option<H256>,
    db: &sp_trie::MemoryDB<Blake2Hasher>,
) -> TrieNode {
    use sp_trie::NodeCodec;
    use trie_db::node::NodePlan;

    if let Ok(node) = NodeCodec::<Blake2Hasher>::decode_plan(bytes) {
        match node {
            NodePlan::Empty => TrieNode::Empty { id: node_id },
            NodePlan::Leaf { partial, value } => TrieNode::Leaf {
                id: node_id,
                nibbles: partial.build(bytes).to_stored(),
                value: Vec::<u8>::from(&bytes[value]),
            },
            NodePlan::Branch { value, children } => TrieNode::Branch {
                id: node_id,
                value: value.map(|range| Vec::<u8>::from(&bytes[range])),
                children: decode_children_recursive(bytes, children, db),
            },
            NodePlan::NibbledBranch {
                partial,
                value,
                children,
            } => TrieNode::NibbledBranch {
                id: node_id,
                nibbles: partial.build(bytes).to_stored(),
                value: value.map(|range| Vec::<u8>::from(&bytes[range])),
                children: decode_children_recursive(bytes, children, db),
            },
            _ => TrieNode::Empty { id: node_id },
        }
    } else {
        TrieNode::Empty { id: None }
    }
}

#[wasm_bindgen]
pub struct JsTrie {
    db: sp_trie::MemoryDB<Blake2Hasher>,
    root: sp_trie::TrieHash<Layout>,
}

#[wasm_bindgen]
impl JsTrie {
    pub fn new() -> Self {
        let mut db = sp_trie::MemoryDB::default();
        let mut root = sp_trie::TrieHash::<Layout>::default();
        sp_trie::TrieDBMut::<Layout>::new(&mut db, &mut root);
        Self { db, root }
    }

    pub fn clear(&mut self) {
        self.db.clear();
        self.root = sp_trie::TrieHash::<Layout>::default();
        sp_trie::TrieDBMut::<Layout>::new(&mut self.db, &mut self.root);
    }

    pub fn insert(
        &mut self,
        key: &js_sys::Uint8Array,
        value: &js_sys::Uint8Array,
    ) -> js_sys::Boolean {
        let key: Vec<u8> = key.to_vec();
        let value: Vec<u8> = value.to_vec();

        match sp_trie::TrieDBMut::<Layout>::from_existing(&mut self.db, &mut self.root) {
            Ok(mut trie) => trie.insert(key.as_slice(), value.as_slice()).is_ok(),
            Err(_) => false,
        }
        .into()
    }

    pub fn remove(&mut self, key: &js_sys::Uint8Array) -> js_sys::Boolean {
        let key: Vec<u8> = key.to_vec();
        match sp_trie::TrieDBMut::<Layout>::from_existing(&mut self.db, &mut self.root) {
            Ok(mut trie) => trie.remove(key.as_slice()).is_ok(),
            Err(_) => false,
        }
        .into()
    }

    pub fn commit(&mut self) -> js_sys::Boolean {
        match sp_trie::TrieDBMut::<Layout>::from_existing(&mut self.db, &mut self.root) {
            Ok(mut trie) => {
                trie.commit();
                true
            }
            Err(_) => false,
        }
        .into()
    }

    pub fn root(&self) -> js_sys::Uint8Array {
        match sp_trie::TrieDB::<Layout>::new(&self.db, &self.root) {
            Ok(trie) => trie.root().as_bytes().into(),
            Err(_) => js_sys::Uint8Array::default(),
        }
    }

    pub fn get(&self, key: &js_sys::Uint8Array) -> JsValue {
        let key: Vec<u8> = key.to_vec();
        match sp_trie::TrieDB::<Layout>::new(&self.db, &self.root) {
            Ok(trie) => match trie.get(&key) {
                Ok(maybe_value) => {
                    if let Some(db_value) = maybe_value {
                        let value: js_sys::Uint8Array = db_value.as_slice().into();
                        value.into()
                    } else {
                        js_sys::Uint8Array::default().into()
                    }
                }
                _ => JsValue::NULL,
            },
            Err(_) => JsValue::NULL,
        }
    }

    pub fn db_values(&self) -> JsValue {
        match sp_trie::TrieDB::<Layout>::new(&self.db, &self.root) {
            Ok(trie) => {
                let root_key = trie.root();
                if let Some(root_data) = self.db.get(&root_key, EMPTY_PREFIX) {
                    let node = decode_recursive(root_data.as_slice(), Some(*root_key), &self.db);
                    return node.encode().into();
                }
            }
            Err(_) => {}
        }
        JsValue::NULL
    }

    pub fn values(&self) -> js_sys::Map {
        let map = js_sys::Map::default();
        self.db.keys().iter().for_each(|key| {
            let value = self.db.get(key.0, EMPTY_PREFIX).unwrap();
            let key: js_sys::Uint8Array = key.0 .0.as_slice().into();
            let value: js_sys::Uint8Array = value.as_slice().into();
            map.set(key.as_ref(), value.as_ref());
        });
        map
    }
}

#[wasm_bindgen]
pub fn blake2_512(x: &js_sys::Uint8Array) -> js_sys::Uint8Array {
    let hash = sp_core::hashing::blake2_512(&x.to_vec());
    hash.as_ref().into()
}

#[wasm_bindgen]
pub fn blake2_256(x: &js_sys::Uint8Array) -> js_sys::Uint8Array {
    let hash = sp_core::hashing::blake2_256(&x.to_vec());
    hash.as_ref().into()
}

#[wasm_bindgen]
pub fn blake2_128(x: &js_sys::Uint8Array) -> js_sys::Uint8Array {
    let hash = sp_core::hashing::blake2_128(&x.to_vec());
    hash.as_ref().into()
}

#[wasm_bindgen]
pub fn twox_64(x: &js_sys::Uint8Array) -> js_sys::Uint8Array {
    let hash = sp_core::hashing::twox_64(&x.to_vec());
    hash.as_ref().into()
}
#[wasm_bindgen]
pub fn twox_128(x: &js_sys::Uint8Array) -> js_sys::Uint8Array {
    let hash = sp_core::hashing::twox_128(&x.to_vec());
    hash.as_ref().into()
}
