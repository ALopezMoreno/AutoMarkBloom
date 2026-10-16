"""Shrink a core flower export into a viewer sample.

Keeps every bud frame at an even index (and t=0), every open frame, and stores each
petal's `mid` and `thick` as little-endian int16 in base64 (`mid_i16`, `thick_i16`),
with the step sizes in a top-level `packing` object. The viewer unpacks this before
validating, so the result is an ordinary flower document again.

    python3 scripts/pack_sample.py flower_11_0.json public/samples/flower_11_0.json
"""
import base64, json, struct, sys

src, dst = sys.argv[1], sys.argv[2]
d = json.load(open(src))
frames = [f for i, f in enumerate(d["frames"]) if f.get("stage") != "bud" or i % 2 == 0 or f["t"] == 0]
mid_max = max(abs(v) for f in frames for p in f["petals"] for v in p["mid"])
thick_max = max(v for f in frames for p in f["petals"] for v in p.get("thick", [0]))
mid_step = mid_max / 32000 or 1.0
thick_step = thick_max / 32000 or 1.0

def pack(values, step):
    return base64.b64encode(struct.pack(f"<{len(values)}h", *(round(v / step) for v in values))).decode()

for f in frames:
    for p in f["petals"]:
        p["mid_i16"] = pack(p.pop("mid"), mid_step)
        if "thick" in p:
            p["thick_i16"] = pack(p.pop("thick"), thick_step)
d["frames"] = frames
d["packing"] = {"int16": True, "mid_step": mid_step, "thick_step": thick_step}
json.dump(d, open(dst, "w"), separators=(",", ":"))
print(f"{len(frames)} frames, mid step {mid_step:.2g}, thick step {thick_step:.2g}")
