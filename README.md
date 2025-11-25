> [!WARNING]
> This repo is deprecated, an updated version is now located at https://github.com/Lohann/merkle-patricia-trie-demo
<div align="center">

  <h1><code>substrate-state-visualizer</code></h1>
</div>

## About
[TODO]

## 🚴 Usage

### 🐑 Install `wasm-pack`

[Learn more about `wasm-pack` here.](https://rustwasm.github.io/book/game-of-life/setup.html)

```
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### 🛠️ Build with `wasm-pack build`

```
wasm-pack build
```

### 🔬 Test in the Browser

```
cd html
npm install
npm start
```

### 🎁 Basic Usage

```javascript
// Insert a value in the trie
trie.insert("0x00aabb", "0xdeadbeef")

// Commit the changes (run after insert or remove)
trie.commit()

// Remove a value
trie.remove("0x00aabb")

// Remove all values from the trie
trie.clear()

// Get the root node hash
trie.root()

// Return all nodes in the trie
trie.db_values()

// Hash functions
wasm.blake2_128("0xdeadbeef")
wasm.blake2_256("0xdeadbeef")
wasm.twox_128("0xdeadbeef")
wasm.twox_64("0xdeadbeef")
```

