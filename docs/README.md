# Model documentation

`automark.tex` explains how each stage of the model works, with the maths, as
implemented in the code. Build it with `make` in this folder (needs `latexmk`
and TeX Live with `latex-extra`); CI builds the PDF on every push and pull
request and attaches it as the `automark-docs` artifact.

`make html` (needs pandoc) writes `build/automark.html`, a phone-friendly page
with the same content: `web/flatten.py` flattens the LaTeX and renders each
TikZ figure to SVG, pandoc converts the text, and `web/build.py` fills
`web/page.tpl.html`. Maths is typeset in the browser by MathJax. This page is
what gets published as the readable web version.

| File | Stage |
|---|---|
| `sections/status.tex` | Which stages are built, being reworked or only designed |
| `sections/overview.tex` | Aims, pipeline, why shells and sheets |
| `sections/seeds.tex` | PRNG, random streams, species and traits |
| `sections/shells.tex` | Nested egg-shaped shells and layer slots |
| `sections/automaton.tex` | Stochastic automaton on each shell |
| `sections/sheets.tex` | Thick spline petal sheets |
| `sections/packing.tex` | Packing energy, exact gradient, FIRE relaxation |
| `sections/output.tex` | The flower JSON shared with the viewer |
| `sections/flowering.tex` | Per-flower Markov chain |
| `sections/colour.tex` | Logistic-map colour |
| `sections/determinism.tex` | Same seed, same flower on every platform |
| `sections/future.tex` | Deferred rules and future design goals |
| `macros.tex` | Shared notation and the sync date and commit |
| `figures/ca-*.tex` | Worked automaton example, generated (see below) |

## Keeping it in sync

Each section opens with a `\stagebox{status}{sources}` line naming the code it
describes. `check-sync.sh [base]` lists sections whose sources changed since
`base` while the section did not, and CI prints the same list as warnings on
pull requests. When a stage changes, update its section and its status, then
bump `\docsynced` and `\docsyncedcommit` in `macros.tex`.

## Generated figures

The worked automaton example (seed 11, flower 0) is recorded from the model.
`tools/ca-trace` re-runs `core/src/ca.rs` step by step with the same random
stream and fails if its grids differ from the model's; `tools/ca_figures.py`
turns that record into `figures/ca-*.tex`:

    cd tools/ca-trace && cargo run --release -- 11 0 > trace.json
    python3 ../ca_figures.py trace.json ../../figures

Rerun both when `ca.rs`, the shells or the peony traits change, and update the
numbers in the example's tables from the same record.
