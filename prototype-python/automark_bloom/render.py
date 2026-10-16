"""2D top-down matplotlib rendering of a grown flower.

Color is deliberately flat (Phase 3 is not built): one pink, shaded only by
geometry, so any visible difference between flowers comes from Phases 1-2.
"""
import numpy as np
from matplotlib.patches import Polygon, Circle
from matplotlib.colors import to_rgb
from .bud_ca import NR, NS, EMPTY, CENTER

PINK = np.array(to_rgb("#e98aa8"))
DEEP = np.array(to_rgb("#b84a72"))
LIGHT = np.array(to_rgb("#fbd3df"))


def _outline(L, W, ruffle, phase, n=40):
    t = np.linspace(0, 1, n)
    half = t ** 0.45 * np.clip(1 - t ** 6, 0, None) ** 0.35    # obovate, rounded tip
    half = half / half.max() * W / 2
    x = np.concatenate([L * t, L * t[::-1]])
    y = np.concatenate([half, -half[::-1]])
    tip = np.clip((x / L - 0.55) / 0.45, 0, 1)
    x = x + ruffle * np.sin(9 * y / max(W, 1e-6) * np.pi + phase) * tip
    return x, y


def draw_flower(ax, flower, ruffle=0.04, rng=None):
    rng = rng or np.random.default_rng(0)
    L, W, C = flower.length, flower.width, flower.curl
    proj = np.maximum(L * np.cos(C), 0.18 * L)          # cupped petals foreshorten
    Wp = W * (1 - 0.35 * np.clip(np.sin(C), 0, 1))
    phases = rng.uniform(0, 2 * np.pi, len(L))
    order = np.argsort([-p.ring0 for p in flower.bud.petals], kind="stable")
    for i in order:
        a, b = flower.angle[i], flower.base[i]
        x, y = _outline(proj[i], Wp[i], ruffle, phases[i])
        x = x + b
        ca, sa = np.cos(a), np.sin(a)
        pts = np.column_stack([x * ca - y * sa, x * sa + y * ca])
        depth = 1 - flower.bud.petals[i].ring0 / NR          # inner -> 1
        cup = np.clip(np.sin(C[i]), 0, 1)
        col = PINK * (1 - 0.35 * depth) + LIGHT * 0.35 * depth
        col = col * (1 - 0.15 * cup) + DEEP * 0.15 * cup
        ax.add_patch(Polygon(pts, closed=True, fc=col, ec=DEEP * 0.85, lw=0.35, alpha=0.97))
        xs, ys = _outline(proj[i] * 0.45, Wp[i] * 0.5, 0, 0, n=20)   # base shadow
        xs = xs + b
        ps = np.column_stack([xs * ca - ys * sa, xs * sa + ys * ca])
        ax.add_patch(Polygon(ps, closed=True, fc=DEEP, ec="none", alpha=0.18))
    c = flower.bud.center_rings
    r_c = 0.10
    n_st = 6 * c + 8
    th = np.linspace(0, 2 * np.pi, n_st, endpoint=False) + 0.3
    for k, t in enumerate(th):
        rr = r_c * (0.55 + 0.4 * ((k * 7) % 5) / 4)
        ax.add_patch(Circle((rr * np.cos(t), rr * np.sin(t)), 0.012, fc="#f2c94c", ec="#b88a1b", lw=0.3, zorder=5))
    for k in range(c):
        t = 2 * np.pi * k / c
        ax.add_patch(Circle((0.03 * np.cos(t), 0.03 * np.sin(t)), 0.02, fc="#9bbf6a", ec="#5d7d39", lw=0.4, zorder=6))
    ax.set_xlim(-1.22, 1.22); ax.set_ylim(-1.22, 1.22)
    ax.set_aspect("equal"); ax.axis("off")


def draw_bud_grid(ax, bud):
    """Polar view of the CA ownership grid."""
    owner = bud.owner
    ids = np.unique(owner[owner >= 0])
    rng = np.random.default_rng(1)
    palette = {pid: rng.uniform(0.25, 0.95, 3) for pid in ids}
    dth = 2 * np.pi / NS
    for r in range(NR):
        for s in range(NS):
            o = owner[r, s]
            col = palette[o] if o >= 0 else (np.array(to_rgb("#f2c94c")) if o == CENTER else np.ones(3) * 0.93)
            th0 = s * dth
            th = np.linspace(th0, th0 + dth, 4)
            pts = np.concatenate([np.column_stack([r * np.cos(th), r * np.sin(th)]),
                                  np.column_stack([(r + 1) * np.cos(th[::-1]), (r + 1) * np.sin(th[::-1])])])
            ax.add_patch(Polygon(pts, closed=True, fc=col, ec="none"))
    for p in bud.petals:
        ax.plot((p.ring0 + 0.5) * np.cos(p.angle), (p.ring0 + 0.5) * np.sin(p.angle), "k.", ms=1.5)
    ax.set_xlim(-NR, NR); ax.set_ylim(-NR, NR); ax.set_aspect("equal"); ax.axis("off")
