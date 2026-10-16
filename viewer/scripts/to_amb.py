"""Convert a core flower export (JSON) into a compact .amb file (see FORMAT.md, "Compact files").

    python3 scripts/to_amb.py flower.json flower.amb          # binary
    python3 scripts/to_amb.py flower.json flower.amb.txt      # same bytes, base64 (for hosts that only serve text)

Keeps every frame. Petal shapes are rounded to int16 steps far below a petal's thickness,
stored as frame-to-frame differences, byte-shuffled and deflated.
"""
import base64, json, struct, sys, zlib

import numpy as np

src, dst = sys.argv[1], sys.argv[2]
d = json.load(open(src))
n = d["grid"]["nu"] * d["grid"]["nv"]
F, P = len(d["frames"]), len(d["petals"])
mid = np.array([[p.pop("mid") for p in f["petals"]] for f in d["frames"]], dtype=np.float64).reshape(F, P, n * 3)
thick = np.array([[p.pop("thick") for p in f["petals"]] for f in d["frames"]], dtype=np.float64).reshape(F, P, n)
mid_step = float(np.abs(mid).max()) / 32000 or 1.0
thick_step = float(thick.max()) / 32000 or 1.0


def plane(a, step):
    q = np.round(a / step).astype(np.int16)
    delta = np.concatenate([q[:1], np.diff(q, axis=0)]).astype(np.int16)  # wraps mod 2^16; the reader wraps too
    b = delta.astype("<i2").view(np.uint8).reshape(-1, 2)
    return b[:, 0].tobytes() + b[:, 1].tobytes()  # low bytes, then high bytes: deflates far better


d["encoding"] = {"codec": "amb1", "mid_step": mid_step, "thick_step": thick_step}
header = json.dumps(d, separators=(",", ":")).encode()
payload = zlib.compress(plane(mid, mid_step) + plane(thick, thick_step), 9)
blob = b"AMB1" + struct.pack("<I", len(header)) + header + payload
open(dst, "wb").write(base64.b64encode(blob) if dst.endswith(".txt") else blob)
print(f"{F} frames, {P} petals: {len(blob) / 1e6:.1f} MB ({dst})")
