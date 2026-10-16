// Reader for compact .amb flower files (FORMAT.md, "Compact files"); scripts/to_amb.py writes them.

const MAGIC = "AMB1";
const MAGIC_B64 = "QU1CM"; // how base64 of "AMB1…" always begins, for .amb.txt copies

/** True if `bytes` is an .amb file, raw or base64. */
export function isAmb(bytes: Uint8Array): boolean {
  const head = String.fromCharCode(...bytes.subarray(0, 6));
  return head.startsWith(MAGIC) || head.startsWith(MAGIC_B64);
}

async function inflate(bytes: Uint8Array): Promise<Uint8Array> {
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Undo the byte shuffle and the frame-to-frame deltas of one int16 plane; scale into Float32Arrays. */
function plane(raw: Uint8Array, offset: number, frames: number, perFrame: number, step: number): Float32Array[] {
  const count = frames * perFrame;
  const lo = raw.subarray(offset, offset + count), hi = raw.subarray(offset + count, offset + 2 * count);
  const out: Float32Array[] = [];
  const acc = new Int16Array(perFrame);
  for (let f = 0; f < frames; f++) {
    const vals = new Float32Array(perFrame);
    for (let j = 0, k = f * perFrame; j < perFrame; j++, k++) {
      acc[j] += (lo[k] | (hi[k] << 8)) << 16 >> 16; // Int16Array wraps mod 2^16, matching the writer
      vals[j] = acc[j] * step;
    }
    out.push(vals);
  }
  return out;
}

/** Decode an .amb file into an ordinary flower document (petal shapes as Float32Arrays). */
export async function decodeAmb(bytes: Uint8Array): Promise<any> {
  if (!String.fromCharCode(...bytes.subarray(0, 4)).startsWith(MAGIC)) {
    const bin = atob(new TextDecoder().decode(bytes).trim());
    bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const headerLen = view.getUint32(4, true);
  const doc = JSON.parse(new TextDecoder().decode(bytes.subarray(8, 8 + headerLen)));
  const enc = doc.encoding;
  if (enc?.codec !== "amb1") throw new Error(`unknown .amb codec ${enc?.codec}`);
  const raw = await inflate(bytes.subarray(8 + headerLen));
  const n = doc.grid.nu * doc.grid.nv, F = doc.frames.length, P = doc.petals.length;
  const mids = plane(raw, 0, F, P * n * 3, enc.mid_step);
  const thicks = plane(raw, 2 * F * P * n * 3, F, P * n, enc.thick_step);
  doc.frames.forEach((f: any, fi: number) =>
    f.petals.forEach((p: any, pi: number) => {
      p.mid = mids[fi].subarray(pi * n * 3, (pi + 1) * n * 3);
      p.thick = thicks[fi].subarray(pi * n, (pi + 1) * n);
    }),
  );
  delete doc.encoding;
  return doc;
}
