# Bud performance: where the time goes and possible speedups

A canonical peony bud takes 15–40 s natively and about 2x that in wasm. Almost all of it
is FIRE relaxation: `(1 + growth_stages) × relax_iterations + settle_iterations`, about
3,850 energy evaluations for a 13-stage bud, each costing about 8–10 ms for 55 petals.

## What the relaxation is actually doing (measured, seed 11 flower 0)

- **Early stages converge.** Stages 0–2 have most of their energy drop done by
  iteration 40 and are fully done by about 120.
- **Late stages never converge.** From stage 3 on, after 250 iterations only 50–80% of
  the stage's energy drop has happened, and energy is still falling steadily. The final
  600-iteration settle only gets about 20% of the way. So stopping on a convergence
  test saves only about 10%, because the late stages never pass it.
- **The cause is stiffness.** FIRE restarts about every 10 iterations, and the mean time
  step is about 0.001 against `dt_max` = 0.03. The step-length clamp never fires, so the
  stiffest modes (most likely stretch springs and contact) set the time step, and the
  soft folding modes crawl.
- **The output is under-relaxed.** The canonical bud is a snapshot of an ongoing
  relaxation, not an equilibrium. This is also why the preview preset (40/80 iterations)
  looks so similar: neither one finishes.

## Possible speedups

| Idea | Expected gain | Changes seeds' output? | Notes |
|---|---|---|---|
| **Precondition the optimizer** (per-node mass from the diagonal of the Hessian, or L-BFGS instead of FIRE) | Large: likely 5–20x fewer iterations to the same energy | Yes | Attacks the measured cause. Also brings the bud closer to real equilibrium, so the "number of stages" gets a cleaner meaning. Contact is C1, so L-BFGS needs a line search that tolerates kinks. |
| **Grow while relaxing**: rest lengths ramp continuously inside one long relaxation, and the growth automaton re-reads pressure every N iterations | 4–8x | Yes | Keeps the pressure feedback and avoids paying for a full relaxation per stage. Closer to biology, since growth is continuous. Combines well with preconditioning. |
| **Convergence-based stopping**: stop a relaxation when energy over the last 20 iterations drops by less than a tolerance | About 10% today | Yes | Only helps the early stages until the stiffness problem is fixed. Worth adding together with preconditioning. |
| **Coarse-to-fine**: early stages on a coarser petal grid, refined for the late crumpling stages | 2–3x | Yes | More code (grid transfer of positions, springs and growth state). |
| **Softer stretch springs** (petals slightly extensible) | 2–4x | Yes | Cheap to try, but it changes the physics: petals would stretch instead of buckling. |
| **Parallel contact search** (threads) | 2–4x natively, less in wasm | Only if done carefully | Determinism rule: no threads in anything that sums floats. Candidate finding can be parallel if the result is sorted. Energy and gradient sums must stay in a fixed order. |
| **Preview preset** (`PackSettings::preview()`, done) | About 6x | Preview only | Already merged; canonical output is unchanged. |

Any change that alters canonical output needs the golden digests re-blessed
(`BLESS=1 cargo test --release -p automark-checks`).
