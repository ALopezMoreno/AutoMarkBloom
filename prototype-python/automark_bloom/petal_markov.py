"""Phase 2: per-flower petal growth as a discrete Markov chain.

Each petal's state is three bin indices: length, width, curl. Every growth
step, each index moves -1, 0 or +1. The move probabilities depend only on the
current state (Markov), through:

* a drift toward a target bin set by the petal's place in the bud (outer
  petals target longer, wider, flatter; inner ones shorter and cupped);
* packing pressure from overlapping neighbors in nearby layers, computed from
  the current state, which suppresses widening and pushes curl up, so crowded
  petals bend instead of spreading.

Petals start as a closed bud (shortest, narrowest, fully curled) and open
over the steps. The flower RNG drives the walk, so sibling flowers share
targets but take different paths.
"""
from dataclasses import dataclass
import numpy as np
from .bud_ca import NR

L_BINS = np.linspace(0.08, 0.72, 14)              # length, flower radius units
W_BINS = np.linspace(0.06, 0.60, 12)              # width at widest point
C_BINS = np.deg2rad(np.linspace(-20, 100, 13))    # curl: + cups inward, - reflexes
STEPS = 45


@dataclass
class Flower:
    bud: object
    base: np.ndarray        # base radius per petal
    angle: np.ndarray
    state: np.ndarray       # (n, 3) final bin indices
    history: np.ndarray     # (STEPS+1, n, 3)

    @property
    def length(self): return L_BINS[self.state[:, 0]]
    @property
    def width(self): return W_BINS[self.state[:, 1]]
    @property
    def curl(self): return C_BINS[self.state[:, 2]]


def _sig(x):
    return 1.0 / (1.0 + np.exp(-x))


def _targets(bud):
    c = bud.center_rings
    span_r = NR - c
    r0 = np.array([p.ring0 for p in bud.petals], float)
    r1 = np.array([p.ring1 for p in bud.petals], float)
    span = np.array([p.span for p in bud.petals])
    pos0 = (r0 - c) / span_r
    pos1 = (r1 + 1 - c) / span_r
    base = 0.08 + 0.42 * pos0
    tl = 0.22 + 0.48 * pos1
    tw = np.clip(span * (r1 + 1) / NR * 1.5, W_BINS[0], W_BINS[-1])
    tc = np.deg2rad(80 - 95 * pos1)

    def to_bin(v, bins):
        return np.interp(v, bins, np.arange(len(bins)))
    return base, to_bin(tl, L_BINS), to_bin(tw, W_BINS), tc, r0


def _pressure(base, angle, L, W, C, r0):
    """Fractional angular overlap with neighbors within two layers."""
    reach = base + np.maximum(L * np.cos(C), 0.15 * L) * 0.6
    hw = np.arctan2(W / 2, reach)
    d = np.abs((angle[:, None] - angle[None, :] + np.pi) % (2 * np.pi) - np.pi)
    near = (np.abs(r0[:, None] - r0[None, :]) <= 2) & ~np.eye(len(angle), dtype=bool)
    ov = np.clip(hw[:, None] + hw[None, :] - d, 0, None) * near
    return ov.sum(1) / np.maximum(hw, 1e-6)


def _step(i, target, rng, g_up, g_down, gain, up_scale=1.0):
    d = target - i
    p_up = g_up * _sig(gain * d) * up_scale
    p_dn = g_down * _sig(-gain * d)
    u = rng.random(len(i))
    move = np.where(u < p_up, 1, np.where(u < p_up + p_dn, -1, 0))
    return i + move


def grow_flower(bud, rng) -> Flower:
    base, tl, tw, tc_rad, r0 = _targets(bud)
    angle = np.array([p.angle for p in bud.petals])
    n = len(angle)
    s = np.zeros((n, 3), int)
    s[:, 2] = len(C_BINS) - 1                    # closed bud: fully curled
    hist = [s.copy()]
    for _ in range(STEPS):
        L, W, C = L_BINS[s[:, 0]], W_BINS[s[:, 1]], C_BINS[s[:, 2]]
        P = _pressure(base, angle, L, W, C, r0)
        tc = np.interp(tc_rad + np.deg2rad(30) * np.minimum(P, 1.0),
                       C_BINS, np.arange(len(C_BINS)))
        s[:, 0] = _step(s[:, 0], tl, rng, 0.55, 0.20, 1.5)
        s[:, 1] = _step(s[:, 1], tw, rng, 0.50, 0.20, 1.5, up_scale=np.exp(-1.5 * P))
        s[:, 2] = _step(s[:, 2], tc, rng, 0.35, 0.45, 1.5)
        s[:, 0] = np.clip(s[:, 0], 0, len(L_BINS) - 1)
        s[:, 1] = np.clip(s[:, 1], 0, len(W_BINS) - 1)
        s[:, 2] = np.clip(s[:, 2], 0, len(C_BINS) - 1)
        hist.append(s.copy())
    return Flower(bud, base, angle, s, np.array(hist))
