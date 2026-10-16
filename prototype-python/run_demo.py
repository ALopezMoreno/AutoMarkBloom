"""Render sibling flowers from two plants, the bud CA grids, and Markov traces."""
import sys
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

sys.path.insert(0, str(Path(__file__).parent))
from automark_bloom import PlantSeed, grow_bud, grow_flower
from automark_bloom.render import draw_flower, draw_bud_grid
from automark_bloom.petal_markov import L_BINS, W_BINS, C_BINS

OUT = Path(__file__).parent / "output"
OUT.mkdir(exist_ok=True)
PLANTS = {"Plant A": PlantSeed(11), "Plant B": PlantSeed(2027)}
N_SIB = 4
BG = "#fbf8f3"

grown = {}
for name, seed in PLANTS.items():
    traits = seed.traits()
    bud = grow_bud(traits, seed.plant_rng())
    flowers = [grow_flower(bud, seed.flower_rng(k)) for k in range(N_SIB)]
    grown[name] = (traits, bud, flowers)
    print(f"{name} seed={seed.value} petals={len(bud.petals)}")

# 1. siblings vs plants
fig, axes = plt.subplots(2, N_SIB, figsize=(4 * N_SIB, 8.6), facecolor=BG)
for row, (name, (traits, bud, flowers)) in enumerate(grown.items()):
    for k, f in enumerate(flowers):
        ax = axes[row, k]
        draw_flower(ax, f, ruffle=traits.ruffle, rng=PLANTS[name].flower_rng(100 + k))
        ax.set_title(f"{name} · flower {k + 1}", fontsize=10, color="#555")
    axes[row, 0].text(-1.3, 0, f"{name}\nseed {PLANTS[name].value}\n{len(bud.petals)} petals",
                      ha="right", va="center", fontsize=11, color="#333")
fig.suptitle("Same plant = same bud (Phase 1); each flower = its own Markov walk (Phase 2)",
             fontsize=13, color="#333")
fig.tight_layout(rect=(0.04, 0, 1, 0.96))
fig.savefig(OUT / "siblings_vs_plants.png", dpi=130, facecolor=BG)
plt.close(fig)

# 2. bud CA grids
fig, axes = plt.subplots(1, 2, figsize=(10, 5.3), facecolor=BG)
for ax, (name, (traits, bud, _)) in zip(axes, grown.items()):
    draw_bud_grid(ax, bud)
    ax.set_title(f"{name}: bud CA ({len(bud.petals)} petals)", fontsize=11, color="#333")
fig.suptitle("Phase 1 polar grid: each color is one petal's cells; dots are primordia", fontsize=11, color="#555")
fig.tight_layout()
fig.savefig(OUT / "bud_ca_grids.png", dpi=130, facecolor=BG)
plt.close(fig)

# 3. Markov traces for one outer and one inner petal across siblings
traits, bud, flowers = grown["Plant A"]
r0 = np.array([p.ring0 for p in bud.petals])
picks = {"inner petal": int(np.argmin(r0)), "outer petal": int(np.argmax(r0))}
fig, axes = plt.subplots(2, 3, figsize=(12, 6), facecolor=BG, sharex=True)
labels = [("length", L_BINS), ("width", W_BINS), ("curl (deg)", np.rad2deg(C_BINS))]
colors = ["#c2185b", "#1565c0", "#2e7d32", "#ef6c00"]
for row, (pname, pi) in enumerate(picks.items()):
    for col, (lab, bins) in enumerate(labels):
        ax = axes[row, col]
        for k, f in enumerate(flowers):
            ax.step(range(f.history.shape[0]), bins[f.history[:, pi, col]], where="post",
                    color=colors[k], lw=1.2, alpha=0.8, label=f"flower {k + 1}")
        ax.set_title(f"{pname}: {lab}", fontsize=10)
        ax.set_facecolor("white")
axes[0, 0].legend(fontsize=8, frameon=False)
for ax in axes[1]:
    ax.set_xlabel("growth step")
fig.suptitle("Phase 2 Markov walks for the same petal in four sibling flowers (Plant A)", fontsize=12)
fig.tight_layout()
fig.savefig(OUT / "markov_traces.png", dpi=130, facecolor=BG)
plt.close(fig)
print("wrote", sorted(p.name for p in OUT.glob("*.png")))
