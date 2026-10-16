// Live access to the Rust core. Present only when the wasm build exists next
// to the viewer (wasm/pkg); otherwise the page falls back to samples and the stub.

export const coreAvailable = Object.keys(import.meta.glob("../../wasm/pkg/automark_wasm.js")).length > 0;

let worker: Worker | null = null;
let nextId = 0;
const pending = new Map<number, { resolve: (v: { json: string; ms: number }) => void; reject: (e: Error) => void }>();

/** Grows the bud for `seed` (a u64, as a decimal string) and returns its flower JSON. */
export function growBud(seed: string): Promise<{ json: string; ms: number }> {
  if (!worker) {
    worker = new Worker(new URL("./coreWorker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (e) => {
      const { id, json, ms, error } = e.data;
      const p = pending.get(id);
      pending.delete(id);
      if (error) p?.reject(new Error(error));
      else p?.resolve({ json, ms });
    };
  }
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    worker!.postMessage({ id, seed });
  });
}
