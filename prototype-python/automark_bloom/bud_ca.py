"""Phase 1: bud formation as a stochastic cellular automaton on a polar grid.

Grid: NR rings x NS sectors, grown center-out one ring at a time. Each cell is
EMPTY, CENTER (stamen/carpel zone) or owned by a petal id. Per ring:

1. Continuation: a petal cell in ring r-1 extends into ring r with
   continue_prob, until the petal reaches its radial extent.
2. Initiation: empty cells, in random order, become a new petal primordium
   when the summed Gaussian inhibition from existing primordia is below a
   threshold (phyllotaxis-style spacing), with init_prob.
3. Lateral packing: a few sweeps in which each empty cell adopts the owner of
   one of its same-ring neighbors with lateral_prob. Petals compete for gaps,
   so widths come from packing rather than being set directly.

The whole run uses the plant RNG, so every bud on a plant is identical.
"""
from dataclasses import dataclass
import numpy as np

NR, NS = 24, 144
EMPTY, CENTER = -1, -2


@dataclass
class Petal:
    id: int
    ring0: int          # initiation ring (radial layer)
    ring1: int          # last ring reached
    angle: float        # circular-mean angle of its cells (rad)
    span: float         # widest angular span over its rings (rad)
    cells: int


@dataclass
class Bud:
    owner: np.ndarray   # (NR, NS) grid of owners
    petals: list
    center_rings: int


def _cell_xy(r, s):
    rad = r + 1.0
    th = 2 * np.pi * s / NS
    return rad * np.cos(th), rad * np.sin(th)


def grow_bud(traits, rng) -> Bud:
    owner = np.full((NR, NS), EMPTY, dtype=int)
    c = traits.center_rings
    owner[:c] = CENTER
    centers = []        # (x, y) of each primordium
    ring0, extent = [], []
    two_s2 = 2 * traits.inhibit_sigma ** 2

    def inhibition(x, y):
        if not centers:
            return 0.0
        p = np.asarray(centers)
        d2 = (p[:, 0] - x) ** 2 + (p[:, 1] - y) ** 2
        return float(np.exp(-d2 / two_s2).sum())

    for r in range(c, NR):
        if r > c:
            for s in np.flatnonzero(owner[r - 1] >= 0):
                pid = owner[r - 1, s]
                if r - ring0[pid] < extent[pid] and rng.random() < traits.continue_prob:
                    owner[r, s] = pid

        for s in rng.permutation(np.flatnonzero(owner[r] == EMPTY)):
            if owner[r, s] != EMPTY:
                continue
            x, y = _cell_xy(r, s)
            if inhibition(x, y) < traits.inhibit_threshold and rng.random() < traits.init_prob:
                pid = len(centers)
                centers.append((x, y))
                ring0.append(r)
                extent.append(int(rng.integers(traits.extent_min, traits.extent_max + 1)))
                owner[r, s] = pid

        for _ in range(traits.lateral_sweeps):
            row = owner[r]
            new = row.copy()
            for s in np.flatnonzero(row == EMPTY):
                cand = [o for o in (row[(s - 1) % NS], row[(s + 1) % NS]) if o >= 0]
                if cand and rng.random() < traits.lateral_prob:
                    new[s] = cand[int(rng.integers(len(cand)))]
            owner[r] = new

    petals = []
    for pid in range(len(centers)):
        rr, ss = np.nonzero(owner == pid)
        if len(rr) == 0:
            continue
        th = 2 * np.pi * ss / NS
        angle = float(np.arctan2(np.sin(th).mean(), np.cos(th).mean()))
        span = max((owner[r] == pid).sum() for r in np.unique(rr)) * 2 * np.pi / NS
        petals.append(Petal(pid, int(rr.min()), int(rr.max()), angle, float(span), int(len(rr))))
    return Bud(owner, petals, c)
