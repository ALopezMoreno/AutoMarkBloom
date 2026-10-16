# AutoMarkBloom viewer

TypeScript + three.js page to orbit a bud, play its opening step by step, and export glTF.

- `npm install`, then `npm run dev` and open the printed URL. `npm run build` writes a static site to `dist/`.
- With the browser build present (`wasm-pack build wasm --release --target web` from the repo root, which writes `wasm/pkg`), the page grows any seed live with the Rust core in a web worker (`src/core.ts`, `src/coreWorker.ts`). Without it, the page offers the stub generator (`src/stub.ts`) and the sample buds in `public/samples`.
- Load a core export with the "Load core JSON" button, by dropping the file on the page, or with `?src=path/to/flower.json`.
- Bud views: a translucent schematic (petals tinted light to deep rose from inner to outer layers, with outlines), explode the bud (outer layers slide out, inner ones rise), peel outer layers away, and cut a vertical or horizontal section. The section cuts the 3D model and draws a 2D packing schematic: shells dashed, petals as lines at their true thickness, coloured inner to outer (`src/section.ts`).
- `?seed=N&t=0.5` opens the stub at a seed and opening fraction.
- `node checks/live.mjs <out-dir>` grows three seeds with the wasm core and screenshots them; `node checks/phone.mjs <out-dir>` screenshots the phone and desktop layouts.
- `node checks/views.mjs <out-dir>` screenshots each bud view on a sample bud.
- `npm run check -- <out-dir>` renders seeds 7 and 42 at three opening points in headless Chromium and round-trips the animated .glb.

Data format: [FORMAT.md](FORMAT.md). Code: `format.ts` (types and validation), `mesh.ts` (thick petals from mid-surface + thickness),
`export.ts` (static .glb, and animated .glb as morph targets with an "opening" clip, resampled to at most 24 keys), `main.ts` (UI).
