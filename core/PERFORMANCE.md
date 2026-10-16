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
| **Parallel contact search** (threads) | Done natively, see below | No | |

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
- **Exact contact skip and a wider skin** (from the Performance thread's bud-side #23). Each
  contact pair stores a lower bound on its distance with every node at its anchor. Energy
  skips a pair while that bound, less how far its nodes have since moved, is still at least
  the contact distance. With most pairs skipped, a skin of 1.0 cap (was 0.5) costs little
  and means fewer refreshes. Output is byte-identical. Seed 11 from seed to open went from
  39 s to 35 s. A skin of 1.5 cap gave the same time, and 2.0 cap was slower.
- **Faster contact search** (from the Performance thread). The refresh now uses flat
  counting-sort grids instead of a BTreeMap, works out each node's movement once, skips
  triangles whose plane is beyond reach, and merges new pairs into the kept ones instead of
  re-sorting. Output is byte-identical (checked on seeds 11 and 2027). Seed 11 from seed to
  open went from 35 s to 29 s.

## Half the relaxation steps (changes output)

The bud now relaxes 30 iterations per stage and 75 to settle (was 60 and 150). Final energy is
about 15% higher (less relaxed), and overlaps and band excess are unchanged. The crumpled
character is kept (compared on seeds 11 and 2027). The preview preset went from 25/50 to 12/30
so it stays about twice as fast as the default. Golden digests re-blessed.

| Bud, one thread | 60 / 150 | 30 / 75 |
|---|---|---|
| seed 11 | 7.5 s, E 18094, overlap 0.60 t | 4.6 s, E 21245, overlap 0.61 t |
| seed 2027 | 5.5 s, E 22476, overlap 0.63 t | 2.6 s, E 25000, overlap 0.63 t |
| seed 7 | 4.3 s, E 8748, overlap 0.78 t | 2.2 s, E 9492, overlap 0.75 t |

An L-BFGS solver (preconditioned with the same per-node masses, backtracking line search) was
also tried. With the same number of energy evaluations it reached about 15–25% lower energy,
but its longer steps doubled the contact rebuilds, so per second it was no better than FIRE.

## Deterministic threads (output unchanged)

Contact search (bud and opening) and contact detection (which pairs touch, in both energies)
run on every core natively (`core/src/par.rs`). Each call cuts its work into 64 chunks fixed
by the work's size, threads take chunks as they free up, and the results are joined in chunk
order. Gradient and energy sums then run in one thread in pair order, so output is the same
bits on any core count (checked: seed 11 and 2027 flower JSON with frames, from seed to open,
identical to main with 1 and 4 threads; golden digests unchanged). `AUTOMARK_THREADS` sets
the thread count; WebAssembly runs the chunks one after another.

| Seed to open, 4-core cloud machine | before | after |
|---|---|---|
| seed 11 | 18.2 s (bud 7.2 s) | 11.4 s (bud 5.1 s) |
| seed 2027 | 10.5 s | 8.1 s |

## Coarse-to-fine bud (changes output)

The first 70% of growth stages (`PackSettings::coarse_fraction`) relax on every other control
point, a 6×10 grid per petal, then the bud is interpolated back to 11×19 for the late stages,
where the petals crumple, and for the settle. The growth automaton stays on the full grid.
Folds land in different places, but the crumpled character is kept (checked on seeds 11 and
2027). Golden digests re-blessed.

| Bud | before | after |
|---|---|---|
| seed 11 | 10.0 s | 5.5 s |
| seed 2027 | 3.8 s | 2.4 s |

Most of the bud's time is now in the full-resolution stages and settle.
