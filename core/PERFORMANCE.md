# Bud performance: where the time goes and possible speedups

Almost all of a bud's time is relaxation:
`(1 + growth_stages) × relax_iterations + settle_iterations` energy evaluations, each
about 8–10 ms for 55 petals natively. The browser (wasm) is about 2x slower.

## What we measured (seed 11 flower 0 unless noted)

- **Plain FIRE never converged.** With one step size for every node, FIRE restarted
  about every 10 iterations and its time step averaged about 0.001 against
  `dt_max` = 0.03. From stage 3 on, 250 iterations covered only 50–80% of each stage's
  energy drop. Convergence-based stopping would therefore have saved only about 10%.
- **Per-node masses fixed most of it.** Each node now gets a mass equal to its local
  stiffness: incident springs, plus contact, envelope and band stiffness wherever those
  are within reach. The envelope radius gets a mass the same way. FIRE runs in
  mass-scaled coordinates, so stiff nodes step small and soft ones step large. With the
  same 250/600 iterations, final energy fell from 3771 to about 1050.

| Settings (relax / settle) | seed 11/0 | seed 11/1 | seed 2027/0 | final energy, 11/0 |
|---|---|---|---|---|
| Old default: plain FIRE, 250 / 600 | 37.6 s | 30.1 s | 14.4 s | 3771 |
| **New default: preconditioned, 60 / 150** | **13.4 s** | **10.2 s** | **5.3 s** | **1753** |
| New preview: preconditioned, 25 / 50 | 6.1 s | 5.7 s | 3.1 s | 2466 |

Timings were taken two runs at a time on one machine. The new default is about 2.8x
faster and about twice as relaxed. The new preview is about 6x faster than the old
default and still more relaxed than it was.

## Possible further speedups

| Idea | Expected gain | Changes seeds' output? | Notes |
|---|---|---|---|
| **Grow while relaxing**: rest lengths ramp continuously inside one long relaxation, and the growth automaton re-reads pressure every N iterations | 3–6x on top of preconditioning | Yes | Keeps the pressure feedback without a full relaxation per stage. Closer to biology, since growth is continuous. |
| **L-BFGS** instead of preconditioned FIRE | Possibly 2–5x fewer iterations | Yes | It learns the stiff and soft directions from recent gradients, so it also helps the slow whole-petal bending modes that a per-node mass cannot reach. Caveats: contact switching makes the energy only C1, so it needs a robust line search (with a fallback to a FIRE step) and a memory reset when the contact list is rebuilt. Each line-search step costs extra energy evaluations. Worth a prototype once the stage loop is settled, and best measured on energy reached per energy evaluation, as above. |
| **Convergence-based stopping**: stop a relaxation when energy over the last 20 iterations falls by less than a tolerance | Small with plain FIRE; worth re-measuring now that stages actually settle | Yes | Cheap to add. |
| **Coarse-to-fine**: early stages on a coarser petal grid, refined for the late crumpling | 2–3x | Yes | More code: grid transfer of positions, springs and growth state. |
| **Parallel contact search** (threads) | 2–4x natively, less in wasm | Not if done carefully | Determinism rule: no threads in anything that sums floats. Candidate finding can run in parallel if its result is sorted. Energy and gradient sums must stay in a fixed order. |

Any change to canonical output needs the golden digests re-blessed
(`BLESS=1 cargo test --release -p automark-checks`).

## Contact pruning and wasm sqrt (output unchanged)

Seed 11's bud spent about two thirds of each energy evaluation on contact pairs that weren't
touching: of about 250k candidates per evaluation only about 2.4k (1%) were in contact.
Each candidate now keeps its distance from when the list was built, and skips the
closest-point test while that distance, less how far its point and triangle have moved,
still can't reach `dmin`. The candidate search also bins nodes into a flat grid
instead of a hash map. Wasm now uses the hardware `sqrt`, which is correctly rounded, so
the bits are the same. Golden digests are unchanged on native and wasm.

| Seed 11 bud | before | after |
|---|---|---|
| native | 15.1 s | 8.5 s |
| wasm (Node) | 28.8 s | 11.8 s |
