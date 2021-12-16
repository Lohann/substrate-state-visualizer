import * as wasm from "wasm-game-of-life";
import Tree from "./trie";
import * as d3 from "d3";

const toHex = (str) => "0x" + Buffer.from(str, "ascii").toString("hex");

const data = [
    // substrate well known keys
    toHex(":code"),
    toHex(":heappages"),
    toHex(":extrinsic_index"),
    toHex(":changes_trie"),
    toHex(":child_storage"),

    // polkadot well known keys; don't seem necessary, but just to make sure
    "0x06de3d8a54d27e44a9d5ce189618f22db4b49d95320d9021994c850f25b8e385",
    "0xf5207f03cfdce586301014700e2c2593fad157e461d71fd4c1f936839a5f1f3e",
    "0x6a0da05ca59913bc38a8630590f2627cb6604cff828a6e3f579ca6c59ace013d",
    "0x6a0da05ca59913bc38a8630590f2627c1d3719f5b0b12c7105c073c507445948",
    "0x6a0da05ca59913bc38a8630590f2627cf12b746dcf32e843354583c9702cc020",
    "0x63f78c98723ddc9073523ef3beefda0c4d7fefc408aac59dbfe80a72ac8e3ce5",
].sort().map((key, i) => ({
    key: new Buffer(key.substr(2), 'hex'),
    value: new Buffer([i]),
}));

function renderRows(data, trie, options) {
    // Render Values Table
    const table = document.getElementById("table-body");
    table.innerHTML = '';

    data.find((d, i) => {
        if (i > 300) {
            return true;
        }
        const tr = document.createElement("tr");
        tr.style.border = '1px solid';
        const indiceTD = document.createElement("th");
        const keyTD = document.createElement("td");
        const valueTD = document.createElement("td");
        const buttonTD = document.createElement("td");
        const button = document.createElement("button");
        indiceTD.style.width = "1%";
        indiceTD.appendChild(document.createTextNode(`${i+1}`));
        keyTD.appendChild(document.createTextNode(d.key.toString("hex")));
        if (options.truncateBigValues && d.value.length > options.truncateBigValues) {
            let splitPosition = Math.ceil(options.truncateBigValues / 2);
            valueTD.appendChild(document.createTextNode(
                d.value.slice(0, splitPosition).toString("hex") +
                "..." +
                d.value.slice(splitPosition, splitPosition + Math.floor(options.truncateBigValues / 2)).toString("hex")
            ));
        } else {
            valueTD.appendChild(document.createTextNode(d.value.toString("hex")));
        }
        button.onclick = function () {
            trie.remove(d.key);
            trie.commit();
            data.splice(i, 1);
            renderRows(data, trie, options);
        }
        button.appendChild(document.createTextNode("remove"));
        buttonTD.appendChild(button);
        tr.appendChild(indiceTD);
        tr.appendChild(keyTD);
        tr.appendChild(valueTD);
        tr.appendChild(buttonTD);
        table.appendChild(tr);
        return false;
    });

    // Render Nodes Table
    let i=0;
    const tableNodes = document.getElementById("table-database");
    if (options.renderStorageNodes) {
        tableNodes.parentNode.style.display = null;
        tableNodes.innerHTML = '';
        i=0;
        let totalBytesStored = 0;
        trie.values().forEach((value, key) => {
            i++;
            key = (new Buffer(key)).toString("hex");
            totalBytesStored += value.length;
            value = " (" + (value.length / 1000).toFixed(4) + "KB)";
            // value = value.toString("hex")
            // if (value.length > 100) {
            //     value = value.substr(0, 40) + "..." +value.substr(-40);
            // }

            const tr = document.createElement("tr");
            const indiceTD = document.createElement("th");
            // indiceTD.style.width = "1%";
            indiceTD.appendChild(document.createTextNode(`${i}`));
            const keyTD = document.createElement("td");
            // keyTD.style.width = "1%";
            const valueTD = document.createElement("td");
            valueTD.style.wordBreak = 'break-all';
            keyTD.appendChild(document.createTextNode(key));
            valueTD.appendChild(document.createTextNode(value));
            tr.appendChild(indiceTD);
            tr.appendChild(keyTD);
            tr.appendChild(valueTD);
            tableNodes.appendChild(tr);
        });
        document.getElementById("storage-size").innerText = `(${totalBytesStored / 1000} KB)`;
    } else {
        tableNodes.parentNode.style.display = 'none';
        tableNodes.innerHTML = '';
    }

    let test = new Buffer([1, 2]);

    // Render Chart
    const content = document.getElementById("content");
    const dbValues = trie.db_values();
    const chart = Tree(dbValues, {
        label: d => {
            let nibbles = d.hasOwnProperty("parent_nibble") ? `${d.parent_nibble}${d.nibbles || ''}` : d.nibbles;
            if (nibbles && nibbles.length > 15) {
                nibbles = nibbles.substr(0,6) + '...' + nibbles.substr(-6)
            }
            let value = d.value;
            if (value) {
                if (value.length > 6) {
                    value = (new Buffer(value.slice(0, 6))).toString('hex') + "...";
                } else {
                    value = (new Buffer(value)).toString('hex');
                }
            }
            return value ? `${nibbles} (${value})` : nibbles;
        },
        title: (d, n) => {
            // hover text
            let hoverText = [d.type + ' '];
            if (d.parent_nibble) {
                hoverText.push(`[${d.parent_nibble}${d.nibbles}]`)
            } else {
                hoverText.push(`[${d.nibbles}]`)
            }
            if (d.value) {
                if (d.value.length > 32) {
                    hoverText.push(" = 0x" + (new Buffer(d.value.slice(0, 32))).toString('hex') + "...");
                } else {
                    hoverText.push(" = 0x" + (new Buffer(d.value)).toString('hex'));
                }
            }
            return hoverText.join('');
        },
        width: Math.max(content.clientWidth, 1152),
        tree: options.cluster ? d3.cluster : d3.tree,
        xScale: options.xScale,
        yScale: options.yScale,
        r: 5,
    })
    content.innerHTML = "";
    content.appendChild(chart);
}

function valToBuffer(value) {
    if (Buffer.isBuffer(value)) {
        return value;
    }
    if (value.constructor === Uint8Array) {
        return new Buffer(value);
    }
    if (typeof value === 'string' || value instanceof String) {
        if (value.startsWith("0x")) {
            return Buffer.from(value.substr(2), 'hex');
        } else {
            return Buffer.from(value, 'ascii');
        }
    }
    if (Number.isInteger(value)) {
        return new Buffer([value]);
    }
    throw Error("can't convert value to Buffer")
}

function trieWrapper(trie, data) {
    return {
        clear: function () {
            trie.clear();
            data.splice(0, data.length);
        },
        commit: function () {
            return trie.commit();
        },
        get: function (key) {
            key = valToBuffer(key);
            return new Buffer(trie.get(key));
        },
        insert: function (key, value) {
            key = valToBuffer(key)
            value = valToBuffer(value)
            let exists = data.find(d => {
                if (d.key.compare(key) === 0) {
                    d.value = value;
                    return true;
                }
                return false;
            })

            if (!exists) {
                data.push({ key, value });
            }

            return trie.insert(key, value);
        },
        remove: function (key) {
            key = valToBuffer(key);
            return trie.remove(key);
        },
        db_values: function () {
            return trie.db_values();
        },
        values: function () {
            return trie.values();
        },
        root: function() {
            return new Buffer(trie.root());
        },
    };
}

const wasmOverride = {
    blake2_128: function (value) {
        value = valToBuffer(value);
        return Buffer.from(wasm.blake2_128(value));
    },
    blake2_256: function (value) {
        value = valToBuffer(value);
        return Buffer.from(wasm.blake2_256(value));
    },
    twox_64: function (value) {
        value = valToBuffer(value);
        return Buffer.from(wasm.twox_64(value));
    },
    twox_128: function (value) {
        value = valToBuffer(value);
        return Buffer.from(wasm.twox_128(value));
    },
}

window.onload = function () {
    // Populate trie
    const trie2 = wasm.JsTrie.new();
    data.forEach(d => {
        trie2.insert(d.key, d.value);
    })
    trie2.commit();

    // Override trie methods
    let trie = trieWrapper(trie2, data);

    // Render
    const options = {
        cluster: false,
        xScale: 5,
        yScale: 1,
        renderStorageNodes: false,
        truncateBigValues: 32,
    }
    renderRows(data, trie, options);

    // Setup Textfield
    const keyTextField = document.getElementById("node-key");
    const valueTextField = document.getElementById("node-value");
    document.getElementById("table-insert").onclick = function () {
        let key = '0x' + keyTextField.value.replaceAll(/\s/g, '');
        let value = '0x' + valueTextField.value.replaceAll(/\s/g, '');
        if (!key) {
            window.alert("Please provide a valid key");
            return
        }
        if (!value) {
            window.alert("Please provide a valid value");
            return;
        }

        trie.insert(key, value);
        trie.commit();
        renderRows(data, trie, options);
    };

    // Setup Cluster Mode
    document.getElementById("mode_tree").onchange = function () {
        options.cluster = false;
        renderRows(data, trie, options)
    }
    document.getElementById("mode_cluster").onchange = function () {
        options.cluster = true;
        renderRows(data, trie, options)
    }

    // Setup Show Storage Nodes Checkbox
    document.getElementById("show_storage_checkbox").onchange = function (event) {
        if (event.currentTarget.checked !== options.renderStorageNodes) {
            options.renderStorageNodes = event.currentTarget.checked;
            renderRows(data, trie, options);
        }
    }

    // Sliders
    const xScaleSlider = document.getElementById("xscale-slider");
    xScaleSlider.value = options.xScale;
    xScaleSlider.onchange = function () {
        options.xScale = parseInt(xScaleSlider.value);
        renderRows(data, trie, options);
    }
    const yScaleSlider = document.getElementById("yscale-slider");
    yScaleSlider.value = options.yScale * 50;
    yScaleSlider.onchange = function () {
        options.yScale = parseInt(yScaleSlider.value) / 50;
        renderRows(data, trie, options);
    }

    // Editor
    const editor = ace.edit("editor");
    editor.setValue(`// allowing access to api, hashing, types, util.
// (async ({ trie, data, wasm }) => {
//   ... any user code is executed here ...
// })();

let pallet  = wasm.twox_128("System");
let storage = wasm.twox_128("Account");

for (let i=0; i<10; i++) {
    let pubkey      = wasm.blake2_256(i);
    let pubkey_hash = wasm.blake2_128(pubkey);
    let slot = Buffer.concat([
        pallet,
        storage,
        pubkey_hash,
        pubkey,
    ]);
    trie.insert(slot, "0xeeee");
}`)
    editor.setTheme("ace/theme/monokai");
    editor.session.setMode("ace/mode/javascript");

    document.getElementById("run-code").onclick = function () {
        const code = `(async (trie, data, wasm, Buffer) => {
            ${editor.getValue()}
        }).call(null, arguments[0], arguments[1], arguments[2], arguments[3])`
        const F=new Function (code);
        F(trie, data, wasmOverride, Buffer);
        trie.commit();
        renderRows(data, trie, options);
    }

    // File reader
    document.getElementById("read-file").onchange = function (event) {
        const file = event.currentTarget.files[0];
        if (file) {
            const reader = new FileReader();
            reader.readAsText(file, "UTF-8");
            reader.onload = function (event) {
                let json;
                try {
                    json = JSON.parse(event.currentTarget.result);
                } catch (exception) {
                    window.alert("Invalid JSON file");
                }
                json = json.genesis.raw.top;
                trie.clear();
                for (let key in json) {
                    trie.insert(key, json[key]);
                }
                trie.commit();
                data.sort((a, b) => a.key.compare(b.key));
                renderRows(data, trie, options);
            }
            reader.onerror = function () {
                window.alert("error reading file");
            }
        }
    }

}
