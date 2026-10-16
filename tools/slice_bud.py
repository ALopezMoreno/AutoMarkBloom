import json, sys, numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import cm
from matplotlib.collections import LineCollection
# usage: slice.py out.png file.json [file.json...]  (one row per file, columns = frames)
out, files = sys.argv[1], sys.argv[2:]
def outline(sh, r, th):
    """(x, z) of the shell of scaled radius r; th = 0 on the +x side, mirrors core/src/shells.rs."""
    c = np.sin(th); p3 = 0.5*(5*c**3 - 3*c); tip = np.where(c > 0, c, 0)**8
    g = 1 + sh.get("egg", 0)*p3 + sh.get("point", 0)*tip
    return r*g*np.cos(th), sh["aspect"]*r*g*np.sin(th)
def segs(m, t, axis=1):
    nu, nv, _ = m.shape; S, W = [], []
    for a in range(nu-1):
        for c in range(nv-1):
            q = [m[a,c], m[a,c+1], m[a+1,c+1], m[a+1,c]]; tt = [t[a,c], t[a,c+1], t[a+1,c+1], t[a+1,c]]
            pts = []
            for k in range(4):
                p0, p1 = q[k], q[(k+1)%4]
                y0, y1 = p0[axis], p1[axis]
                if (y0 <= 0 < y1) or (y1 <= 0 < y0):
                    f = y0/(y0-y1); pts.append((p0+f*(p1-p0), tt[k]+f*(tt[(k+1)%4]-tt[k])))
            if len(pts) == 2:
                S.append([(pts[0][0][0], pts[0][0][2]), (pts[1][0][0], pts[1][0][2])]); W.append((pts[0][1]+pts[1][1])/2)
    return S, W
rows = [json.load(open(f)) for f in files]
nf = max(len(d["frames"]) for d in rows); cols = [0, nf//3, 2*nf//3, nf-1] if nf > 1 else [0]
single = nf == 1
if single:
    fig, ax_row = plt.subplots(1, len(rows), figsize=(4.2*len(rows), 4.9), squeeze=False, facecolor="white")
    axes = [[ax_row[0][i]] for i in range(len(rows))]
else:
    fig, axes = plt.subplots(len(rows), len(cols), figsize=(4.2*len(cols), 4.8*len(rows)), squeeze=False, facecolor="white")
for r, d in enumerate(rows):
    nu, nv = d["grid"]["nu"], d["grid"]["nv"]; L = len(d["shells"]["radius"]); A = d["shells"]["aspect"]
    Emax = max(f.get("channels", {}).get("sepal_radius", f.get("envelope", d["shells"]["envelope"])) for f in d["frames"])
    for c, fi in enumerate(cols):
        fr = d["frames"][min(fi, len(d["frames"])-1)]; ax = axes[r][c]
        for info, p in zip(d["petals"], fr["petals"]):
            m = np.array(p["mid"]).reshape(nu, nv, 3); t = np.array(p["thick"]).reshape(nu, nv)
            S_, W = segs(m, t)
            if not S_: continue
            # linewidth in points: thickness in data units -> points
            ax.add_collection(LineCollection(S_, colors=[cm.RdPu(0.3+0.6*info["layer"]/max(L-1,1))], linewidths=[w*95 for w in W], capstyle="round"))
        th = np.linspace(0, 2*np.pi, 300)
        R = fr.get("channels", {}).get("sepal_radius", fr.get("envelope", d["shells"]["envelope"]))
        ax.plot(*outline(d["shells"], R, th), "--", color="#2e7d32", lw=1)
        ax.plot(*outline(d["shells"], d["shells"]["core"], th), "-", color="#d4a017", lw=1)
        ax.set_xlim(-Emax*1.08, Emax*1.08); ax.set_ylim(-A*Emax*1.3, A*Emax*1.3); ax.set_aspect("equal"); ax.axis("off")
        ax.set_title(f"plant {d['seed']} · flower {d.get('flower_index', 0)}: {len(fr['petals'])} petals" if single else f"seed {d['seed']} · " + ("closed bud" if fi == nf-1 else f"stage {fi}"), fontsize=10)
fig.suptitle("Slice through the bud as it develops (green dashed = sepals, gold = stamens, line width = petal thickness)", fontsize=11)
fig.tight_layout(); fig.savefig(out, dpi=120)
