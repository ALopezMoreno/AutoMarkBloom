# AutoMarkBloom viewer

TypeScript + three.js page to orbit a bud, play its opening step by step, and export glTF.

- `npm install`, then `npm run dev` and open the printed URL. `npm run build` writes a static site to `dist/`.
- With the browser build present (`wasm-pack build wasm --release --target web` from the repo root, which writes `wasm/pkg`), the page grows any seed live with the Rust core in a web worker (`src/core.ts`, `src/coreWorker.ts`). Without it, the page offers the stub generator (`src/stub.ts`) and the sample buds in `public/samples`.
- Load a core export with the "Load core JSON" button, by dropping the file on the page, or with `?src=path/to/flower.json`.
- A green stem and five sepals (`src/plant.ts`, viewer-only for now) wrap the bud, sized to whatever the petals reach, and peel back as it opens.
- Bud views: a translucent schematic (petals tinted light to deep rose from inner to outer layers, with outlines), explode the bud (outer layers slide out, inner ones rise), peel outer layers away, and cut a vertical or horizontal section. The section cuts the 3D model and draws a 2D packing schematic: shells dashed, petals as lines at their true thickness, coloured inner to outer (`src/section.ts`).
- `?seed=N&t=0.5` opens the stub at a seed and opening fraction.
- `node checks/live.mjs <out-dir>` grows three seeds with the wasm core and screenshots them; `node checks/phone.mjs <out-dir>` screenshots the phone and desktop layouts.
- `node checks/views.mjs <out-dir>` screenshots each bud view on a sample bud.
- `node checks/opening.mjs <out-dir>` (after `npm run build`) plays the bundled seed 11 bud-to-open sample and screenshots five steps.
- `npm run check -- <out-dir>` renders seeds 7 and 42 at three opening points in headless Chromium and round-trips the animated .glb.

Data format: [FORMAT.md](FORMAT.md). Code: `format.ts` (types and validation), `mesh.ts` (thick petals from mid-surface + thickness),
`export.ts` (static .glb, and animated .glb as morph targets with an "opening" clip, resampled to at most 24 keys), `main.ts` (UI).

## Bud-to-open sample

The wasm build does not expose the opening stage yet, so `public/samples/flower_11_0.json` is a precomputed native run (`automark --seed 11 --flower 0 --frames --open 12 --json out.json`). The raw export is about 54 MB, so `python3 scripts/pack_sample.py out.json public/samples/flower_11_0.json` keeps every other bud frame plus all open frames and stores positions and thicknesses as base64 int16 (about 15 MB). `format.ts` unpacks it before validating. Files ending in `.gz` are also gunzipped on load.
