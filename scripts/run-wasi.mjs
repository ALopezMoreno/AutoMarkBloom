// Runs a wasm32-wasip1 binary under Node's built-in WASI, so CI needs no extra runtime.
// Usage: node scripts/run-wasi.mjs path/to/program.wasm [args...]
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";
import { argv, exit } from "node:process";

const [path, ...args] = argv.slice(2);
const wasi = new WASI({ version: "preview1", args: [path, ...args], env: {}, returnOnExit: true });
const module = await WebAssembly.compile(await readFile(path));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
exit(wasi.start(instance));
