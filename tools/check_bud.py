import json, sys, numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import cm
from matplotlib.colors import LightSource
files = sys.argv[1:-1]; out = sys.argv[-1]
fig = plt.figure(figsize=(15, 5.2*len(files)), facecolor="white")
ls = LightSource(azdeg=315, altdeg=35)
def outline(sh, r, th):
    """(x, z) of the shell of scaled radius r; th = 0 on the +x side, mirrors core/src/shells.rs."""
    c = np.sin(th); p3 = 0.5*(5*c**3 - 3*c); tip = np.where(c > 0, c, 0)**8
    g = 1 + sh.get("egg", 0)*p3 + sh.get("point", 0)*tip
    return r*g*np.cos(th), sh["aspect"]*r*g*np.sin(th)
def col(layer, L): return cm.RdPu(0.18 + 0.5*layer/max(L-1, 1))
for k, f in enumerate(files):
    d = json.load(open(f)); nu, nv = d["grid"]["nu"], d["grid"]["nv"]
    L = len(d["shells"]["radius"]); fr = d["frames"][0]["petals"]; A = d["shells"]["aspect"]
    data = [(i, np.array(p["mid"]).reshape(nu, nv, 3), np.array(p["thick"]).reshape(nu, nv)) for i, p in zip(d["petals"], fr)]
    for j, cut in enumerate([False, True]):
        ax = fig.add_subplot(len(files), 3, 3*k+j+1, projection="3d")
        for info, m, _ in sorted(data, key=lambda x: -x[0]["layer"]):
            X, Y, Z = m[...,0].copy(), m[...,1].copy(), m[...,2].copy()
            if cut:
                mask = Y < 0
                if mask.all(): continue
                X[mask] = np.nan; Y[mask] = np.nan; Z[mask] = np.nan
            ax.plot_surface(X, Y, Z, color=col(info["layer"], L), linewidth=0.15, edgecolor=(0.4,0.1,0.25,0.25), shade=True, lightsource=ls)
        ax.set_box_aspect((1, 1, A)); ax.axis("off"); ax.view_init(12, -90 if cut else -60)
        ax.set_title(("cut open (front half removed)" if cut else f"seed {d['seed']}: {len(fr)} petals in {L} layers"), fontsize=11)
    ax2 = fig.add_subplot(len(files), 3, 3*k+3)
    for info, m, t in data:
        for a in range(nu):
            y = m[a,:,1]
            for c in range(nv-1):
                if y[c]*y[c+1] <= 0 and y[c] != y[c+1]:
                    g = y[c]/(y[c]-y[c+1]); pt = m[a,c] + g*(m[a,c+1]-m[a,c]); tt = t[a,c] + g*(t[a,c+1]-t[a,c])
                    ax2.add_patch(plt.Circle((pt[0], pt[2]), tt/2, color=col(info["layer"], L)))
    th = np.linspace(0, 2*np.pi, 300)
    for r, s_ in [(d["shells"]["core"], "-"), (d["shells"]["envelope"], "--")]:
        ax2.plot(*outline(d["shells"], r, th), s_, color="#999", lw=0.8)
    E = d["shells"]["envelope"]; ax2.set_aspect("equal"); ax2.axis("off")
    ax2.set_xlim(-E*1.05, E*1.05); ax2.set_ylim(-A*E*1.3, A*E*1.3)
    ax2.set_title("slice: dots are petal thickness, dashed = sepals, solid = stamens", fontsize=10)
fig.tight_layout(); fig.savefig(out, dpi=110)
