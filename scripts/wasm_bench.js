// Times a handler inside the built WASM (V8 via Node, no substreams runtime) on
// one block, and prints a hash of its output so builds can be checked for
// byte-identical results.
//
//   cargo run -p polymarket-substreams-common --example dump_fixture -- /tmp/block.bin
//   node scripts/wasm_bench.js /tmp/block.bin map_all_events a.wasm b.wasm ...
//
// Absolute times differ from the substreams runtime (wazero); compare builds
// against each other, in one run.
const fs = require("fs");
const [blockPath, handler, ...wasms] = process.argv.slice(2);
const block = fs.readFileSync(blockPath);
const ITER = 300;

function load(path) {
  let outLen = -1;
  let inst;
  let capture = false;
  const imports = {
    env: {
      skip_empty_output: () => {},
      output: (p, len) => {
        outLen = len;
        if (capture) outHash = require("crypto").createHash("sha256").update(new Uint8Array(inst.exports.memory.buffer, p, len)).digest("hex").slice(0, 16);
      },
      register_panic: () => { throw new Error("panic in " + path); },
    },
    logger: { println: () => {} },
  };
  inst = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync(path)), imports);
  const e = inst.exports;
  // Handlers borrow their input (lazy: from_raw_parts; owned: decode_ptr), so one
  // buffer serves every call.
  const ptr = e.alloc(block.length);
  new Uint8Array(e.memory.buffer, ptr, block.length).set(block);
  let outHash = "";
  const f = () => {
    e[handler](ptr, block.length);
    return outLen;
  };
  capture = true; f(); capture = false;
  f.hash = outHash;
  return f;
}

const runs = wasms.map((w) => ({ w, f: load(w) }));
for (const r of runs) { for (let i = 0; i < 50; i++) r.len = r.f(); }
const times = Object.fromEntries(runs.map((r) => [r.w, []]));
for (let round = 0; round < 5; round++) {
  for (const r of runs) {
    const t0 = process.hrtime.bigint();
    for (let i = 0; i < ITER; i++) r.f();
    times[r.w].push(Number(process.hrtime.bigint() - t0) / 1e6 / ITER);
  }
}
for (const r of runs) {
  const t = times[r.w].sort((a, b) => a - b);
  console.log(`${r.w.split("/").pop().padEnd(24)} size=${fs.statSync(r.w).size} out=${r.len}B sha256=${r.f.hash} median=${t[2].toFixed(3)}ms min=${t[0].toFixed(3)}ms`);
}
