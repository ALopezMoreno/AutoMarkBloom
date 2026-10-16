// Runs the Rust core (WebAssembly) off the main thread so packing a bud
// doesn't freeze the page.

const mods = import.meta.glob("../../wasm/pkg/automark_wasm.js");
const load = Object.values(mods)[0] as (() => Promise<any>) | undefined;

let ready: Promise<any> | null = null;
function core() {
  if (!load) throw new Error("wasm build not found (run: wasm-pack build wasm --release --target web)");
  ready ??= load().then(async (m: any) => {
    await m.default();
    return m;
  });
  return ready;
}

self.onmessage = async (e: MessageEvent<{ id: number; seed: string }>) => {
  const { id, seed } = e.data;
  try {
    const m = await core();
    const t0 = performance.now();
    const json: string = m.bud_json(BigInt(seed));
    self.postMessage({ id, json, ms: performance.now() - t0 });
  } catch (err) {
    self.postMessage({ id, error: String((err as Error).message ?? err) });
  }
};
