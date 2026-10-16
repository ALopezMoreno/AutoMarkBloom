# Flower data format (`automark-flower`, version 1)

Agreed between the viewer, bud and opening (Markov) threads. One JSON document per flower,
written by the core (`automark --seed N --json out.json`, or `bud_json(seed, flower_index)` in
wasm). The core owns geometry and latents; the viewer only meshes, shades, animates and exports.
Readers must ignore fields they don't know.

```jsonc
{
  "format": "automark-flower",
  "version": 1,
  "seed": "12345",            // plant seed, u64 as a decimal string (JS numbers lose precision)
  "flower_index": 0,          // which flower on that plant; always set (each flower grows its own bud)
  "units": "core_radius",     // 1.0 = CORE_RADIUS
  "up": "+z",                 // core frame; the viewer converts to three.js +y
  "shells": { "aspect": 1.1, "egg": 0.04, "point": 0.05, "core": 1.0, "slot": 0.09, "radius": [1.05, 1.14] },
                              // every shell (core included) is |q| = a * g(c), q = (x, y, z/aspect), c = q_z/|q|,
                              // g(c) = 1 + egg * (5c^3 - 3c)/2 + point * max(c, 0)^8; egg/point missing = 0
  "grid": { "nu": 37, "nv": 21 },   // samples per petal: u = base -> tip, v = one edge -> other
  "petals": [
    { "id": 0, "layer": 0, "theta": 0.0 }   // id = index in this list; layer 0 = innermost shell; theta = attachment angle (rad)
  ],
  "channels": [                              // generic latent channels, declared once
    { "name": "onset",        "scope": "petal",  "dims": 1, "kind": "target" },
    { "name": "petal_type",   "scope": "petal",  "dims": 1, "kind": "fixed", "labels": ["flat", "cupped", "rolled"] },
    { "name": "sepal_radius", "scope": "flower", "dims": 1, "kind": "state" },
    { "name": "colour",       "scope": "point",  "dims": 3, "kind": "state", "range": [0, 1] }
  ],
  "frames": [
    {
      "t": -1.0,              // -1..0 bud development, 0 = closed bud, (0, 1] opening
      "stage": "bud",         // "bud" or "open"
      "channels": { "sepal_radius": 1.84 },          // flower-scope values that changed
      "petals": [
        {
          "mid":   [x, y, z, ...],   // nu*nv evaluated mid-surface points, row-major (u outer, v inner)
          "thick": [h, ...],         // nu*nv full thickness per point (already capped)
          "channels": { "onset": 0.12, "petal_type": 1 }   // petal- and point-scope values that changed
        }
      ]
    }
  ],
  "meta": { }                 // free-form provenance (species, traits, pack report); viewers ignore it
}
```

## Geometry rules
- Every frame lists every petal in the same order as `petals`, with the same `nu x nv`. Fixed
  topology is what lets the bud's development and the opening play back smoothly and export as
  an animated glTF.
- `mid` is the evaluated surface, not spline control points, so readers never need the spline basis.
- Orientation: `d(mid)/du x d(mid)/dv` points away from the bud centre (outer face). The viewer
  offsets by +-thick/2 along it and closes the rim.
- Numbers are rounded by the core (currently 1e-5).

## Channels
- `channels` is a list; names are unique. Each entry has `name`, `scope` (`point`, `petal`,
  `flower`), `dims` (1 or 3), and optionally `range` `[lo, hi]`, `kind` and `labels`.
- `kind` is an open, informational set (`growth`, `drift`, `fixed`, `target`, `state`, ...).
  Readers never branch on it. `labels` names the categories of a categorical channel: value `k`
  means `labels[k]`.
- Values: flower and petal scope hold a bare number when `dims` is 1 and `[x, y, z]` when it is 3.
  Point scope holds a flat array of `nu*nv*dims` numbers, row-major like `mid`.
- Values live in `frames[k].channels` (flower scope) and `frames[k].petals[i].channels` (petal and
  point scope). A frame carries only the channels that changed; a channel absent from a frame keeps
  its last value. Latents that never change (targets, fixed types) appear in the first frame only.
- Adding a channel never changes this format: declare it and emit its values.
- The sepal envelope is the `sepal_radius` flower channel (scaled radius, per frame). Readers fall
  back to a legacy per-frame `envelope`, then `shells.envelope`, then the outermost shell plus half a slot.

## What the viewer does with it
- Meshes thick petals, plays all frames in order (labelled with `stage` and `t`), and exports glTF:
  the current pose, or every frame as morph targets with an "opening" clip.
- Offers every declared channel under "Colour by": scalars on a ramp over `range` (or the observed
  min/max), categorical channels with their `labels`, and dims-3 channels as RGB in [0, 1].

## Compact files (.amb)

A flower document can also be stored as a compact binary file. The viewer reads either form, and `scripts/to_amb.py` converts JSON to `.amb`. The seed 11 run (38 steps, 55 petals, 37×21 grid) is 54 MB as JSON and 5.2 MB as `.amb`, and the browser decodes it in about 0.2 s.

Layout, little-endian:

| Bytes | Content |
|---|---|
| 0–3 | `AMB1` |
| 4–7 | header length H (u32) |
| 8 … 8+H | header: the flower document as UTF-8 JSON, with every frame petal's `mid` and `thick` left out, plus `"encoding": {"codec": "amb1", "mid_step": s, "thick_step": s2}` |
| rest | zlib (RFC 1950) stream holding two planes: first all `mid` values, then all `thick` values |

Each plane is ordered frame, petal, point (and xyz for `mid`). A value is stored as the int16 `round(value / step)` minus the same value in the previous frame (frame 0 holds the value itself), with wrap-around mod 2^16. The plane's bytes are shuffled: all low bytes first, then all high bytes. Readers undo this and multiply by the step. The writer picks the steps so that the largest value maps to 32000, so `mid` is accurate to about 1e-4 core radii, which is far below a petal's thickness (about 0.03).

A `.amb.txt` file is the same bytes in base64, for hosts that only serve text.
