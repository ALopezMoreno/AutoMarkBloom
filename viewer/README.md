# AutoMarkBloom viewer

TypeScript + three.js page to orbit a bud, play its opening step by step, and export glTF.

- `npm install`, then `npm run dev` and open the printed URL. `npm run build` writes a static site to `dist/`.
- Until the Rust core emits flowers, the page shows a stub generator (`src/stub.ts`) that follows the same data format.
- Load a core export with the "Load core JSON" button, by dropping the file on the page, or with `?src=path/to/flower.json`.
- `?seed=N&t=0.5` opens the stub at a seed and opening fraction.
- `npm run check -- <out-dir>` renders seeds 7 and 42 at three opening points in headless Chromium and round-trips the animated .glb.

Data format: [FORMAT.md](FORMAT.md). Code: `format.ts` (types and validation), `mesh.ts` (thick petals from mid-surface + thickness),
`export.ts` (static .glb, and animated .glb as morph targets with an "opening" clip, resampled to at most 24 keys), `main.ts` (UI).
