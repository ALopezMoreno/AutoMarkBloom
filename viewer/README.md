# AutoMarkBloom viewer

TypeScript + three.js page to orbit a bud, play its opening step by step, and export glTF.

- `npm install`, then `npm run dev` and open the printed URL. `npm run build` writes a static site to `dist/`.
- With the browser build present (`wasm-pack build wasm --release --target web` from the repo root, which writes `wasm/pkg`), the page grows any seed live with the Rust core in a web worker (`src/core.ts`, `src/coreWorker.ts`). Without it, the page offers the stub generator (`src/stub.ts`) and the sample buds in `public/samples`.
- Load a core export with the "Load core JSON" button, by dropping the file on the page, or with `?src=path/to/flower.json`.
- `?seed=N&t=0.5` opens the stub at a seed and opening fraction.
- `node checks/live.mjs <out-dir>` grows three seeds with the wasm core and screenshots them; `node checks/phone.mjs <out-dir>` screenshots the phone and desktop layouts.
- `npm run check -- <out-dir>` renders seeds 7 and 42 at three opening points in headless Chromium and round-trips the animated .glb.

Data format: [FORMAT.md](FORMAT.md). Code: `format.ts` (types and validation), `mesh.ts` (thick petals from mid-surface + thickness),
`export.ts` (static .glb, and animated .glb as morph targets with an "opening" clip, resampled to at most 24 keys), `main.ts` (UI).
