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

## Opening (stage 2)

Seed 11 flower 0, 24 frames, measured natively:

- **Per-node masses don't help the opening.** With the bud's preconditioning applied to the
  opening's FIRE, the opening took 45–60 s instead of 41 s, and final energy didn't improve.
  The opening isn't stiffness-bound. Each frame warm-starts from the previous one moved by the
  change in pose, and energy plateaus where the pose tether, bending and contact disagree.
  Larger steps only caused more contact rebuilds.
- **Contact rebuilds are about two thirds of the opening's time.** There are about 290
  rebuilds at about 65 ms each, roughly 800k exact sample-triangle tests per rebuild. Binning
  triangles into a finer dense grid and adding a bounding-sphere pre-test both gave identical
  candidates at the same speed. The nearby pairs are genuinely near, so what costs is the
  number of rebuilds.
- **Fewer iterations.** Energy has settled well before 120 iterations, so the default is now
  60 iterations with `dt_max` 0.06:

| Opening relax settings | time | final energy | final max overlap |
|---|---|---|---|
| 120 iterations, dt_max 0.03 (old) | 41.4 s | 308.6 | 0.74 t |
| **60, 0.06 (new default)** | **27.2 s** | **313.8** | **0.68 t** |
| 40, 0.1 | 23.8 s | 308.6 | 0.74 t (worst 0.78 t) |
- **Per-petal contact refresh.** Contact pairs are now refreshed only for petals that moved
  more than half the Verlet skin, together with any petal that moved more than a quarter of
  it, so refreshes come in batches. Kept pairs stay exact because their two petals have
  together moved less than the skin. Output is byte-identical. Seed 11 from bud to open went
  from 47 s to 42.5 s. Widening the skin or changing the batching threshold made no
  difference, so what's left is mostly the energy evaluations themselves.
- **Continuous settle.** FIRE's velocity and step size now carry over from one frame to the
  next (`RelaxSettings::carry`), so the flower keeps moving as it grows instead of starting
  from rest every frame. That lets each frame settle in 20 iterations instead of 60. Seed 11
  on the retuned bud (#20, #22):

| Opening settle | whole flower | final max overlap | change from 60 iterations (RMS / max) |
|---|---|---|---|
| 60 iterations, restarting each frame (old) | 53 s | 0.60 t | — |
| 60, carried over | 55 s | 0.70 t | 0.39 t / 3.0 t |
| **20, carried over (new default)** | **39 s** | **0.62 t** | **0.52 t / 3.4 t** |
| 10, carried over | 33 s | 0.85 t | 0.68 t / 4.8 t |
| no settling, pose only | 21 s | 0.91 t | 1.00 t / 11.9 t |

  The 60-iteration carried-over row shows the solver's own noise: about 0.4 t RMS. At 20
  iterations the change is barely above that, and every layer's opening angle and incurve
  is unchanged. Blending the targets over several settling steps within a frame didn't help.
