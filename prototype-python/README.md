# Automark Bloom prototype (Phases 1-2)

`python3 run_demo.py` writes to `output/`:

- `siblings_vs_plants.png`: 4 sibling flowers from each of two plants.
- `bud_ca_grids.png`: the Phase 1 CA grid for each plant.
- `markov_traces.png`: Phase 2 length/width/curl walks of one inner and one outer petal across siblings.

Modules: `seeds.py` (plant seed -> traits, flower seeds), `bud_ca.py` (Phase 1),
`petal_markov.py` (Phase 2), `render.py` (2D matplotlib). Needs numpy + matplotlib.
Phase 3 (logistic-map color) is not built; color is one flat pink shaded by geometry.
