# Bud / flower data format (draft v1, proposed by the viewer thread)

One JSON document (or the same object returned from wasm via serde) per flower.
The core owns the geometry; the viewer only meshes, shades, animates and exports.

```jsonc
{
  "format": "automark-flower",
  "version": 1,
  "seed": "12345",             // u64 as a decimal string (JS numbers lose precision)
  "flower_index": null,         // null for the bud stage; per-flower index once flowering exists
  "units": "core_radius",       // 1.0 = CORE_RADIUS
  "up": "+z",                   // core frame; the viewer converts to three.js +y
  "shells": { "aspect": 1.4, "core": 1.0, "envelope": 2.1, "radius": [1.05, 1.15] },
  "grid": { "nu": 12, "nv": 9 },   // samples per petal: u = base -> tip, v = one edge -> other
  "petals": [
    { "id": 0, "layer": 0, "theta": 0.0 }   // layer: 0 = innermost shell; theta: attachment angle (rad)
  ],
  "frames": [
    {
      "t": 0.0,                 // 0 = closed bud; increasing through opening (steps, or 0..1)
      "petals": [
        {
          "mid":   [x, y, z, x, y, z, ...],  // nu*nv evaluated mid-surface points, row-major (u outer, v inner)
          "thick": [h, h, ...]               // nu*nv full thickness at each point (already capped)
        }
      ]
    }
  ]
}
```

Rules
- Every frame lists every petal in the same order as `petals`, with the same `nu x nv`. Fixed
  topology is what lets the opening play back smoothly and export as an animated glTF.
- `mid` is the evaluated surface, not spline control points, so the viewer never needs the
  spline basis. If the core wants finer meshes it raises `nu/nv`.
- Orientation: the normal `d(mid)/du x d(mid)/dv` should point away from the bud centre
  (outer face). The viewer offsets by +-thick/2 along it and closes the rim.
- The bud stage alone is just `frames` with one entry.
- Optional later: `"color": [r,g,b, ...]` per point per frame (logistic-map colour stage).

The viewer builds solid meshes and exports glTF (static frame, or all frames as morph
targets / keyframed animation) from this, so the core's own glTF writer can stay a simple
static export for engines, or be dropped.
