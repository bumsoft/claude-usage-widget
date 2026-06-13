// Generates src-tauri/app-icon.png (1024x1024) with no image dependencies.
// A Claude-terracotta rounded tile with a white usage-gauge ring.
// Run `npm run icon` afterwards to derive the platform icon set.
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const SIZE = 1024;
const PAD = 96;
const RADIUS = 200;
const CX = SIZE / 2;
const CY = SIZE / 2;
const R_OUTER = 300;
const R_INNER = 212;
const ARC = 0.72; // fraction of the ring that is "filled" (bright)

const TOP = [226, 146, 108];
const BOT = [190, 92, 58];

function clamp01(v) {
  return v < 0 ? 0 : v > 1 ? 1 : v;
}

function roundRectInside(px, py) {
  const halfW = (SIZE - 2 * PAD) / 2;
  const halfH = (SIZE - 2 * PAD) / 2;
  const dx = Math.max(Math.abs(px - CX) - (halfW - RADIUS), 0);
  const dy = Math.max(Math.abs(py - CY) - (halfH - RADIUS), 0);
  return Math.hypot(dx, dy) - RADIUS <= 0;
}

// Returns [r,g,b,a] in 0..255 for a single sample point.
function sample(px, py) {
  if (!roundRectInside(px, py)) return [0, 0, 0, 0];

  const t = clamp01((py - PAD) / (SIZE - 2 * PAD));
  let r = TOP[0] + (BOT[0] - TOP[0]) * t;
  let g = TOP[1] + (BOT[1] - TOP[1]) * t;
  let b = TOP[2] + (BOT[2] - TOP[2]) * t;

  const dist = Math.hypot(px - CX, py - CY);
  if (dist <= R_OUTER && dist >= R_INNER) {
    let angle = Math.atan2(px - CX, -(py - CY)); // 0 at top, clockwise
    if (angle < 0) angle += Math.PI * 2;
    const progress = angle / (Math.PI * 2);
    const a = progress <= ARC ? 0.96 : 0.2;
    r = r + (255 - r) * a;
    g = g + (255 - g) * a;
    b = b + (255 - b) * a;
  }
  return [r, g, b, 255];
}

// 2x2 supersampling for smoother edges.
function pixel(x, y) {
  let r = 0,
    g = 0,
    b = 0,
    a = 0;
  for (const oy of [0.25, 0.75]) {
    for (const ox of [0.25, 0.75]) {
      const [sr, sg, sb, sa] = sample(x + ox, y + oy);
      const af = sa / 255;
      r += sr * af;
      g += sg * af;
      b += sb * af;
      a += sa;
    }
  }
  const aAvg = a / 4;
  if (aAvg === 0) return [0, 0, 0, 0];
  // un-premultiply
  const cov = aAvg / 255;
  return [
    Math.round(r / 4 / cov),
    Math.round(g / 4 / cov),
    Math.round(b / 4 / cov),
    Math.round(aAvg),
  ];
}

// Build raw RGBA scanlines with PNG "None" filter byte per row.
const raw = Buffer.alloc(SIZE * (1 + SIZE * 4));
let p = 0;
for (let y = 0; y < SIZE; y++) {
  raw[p++] = 0; // filter: None
  for (let x = 0; x < SIZE; x++) {
    const [r, g, b, a] = pixel(x, y);
    raw[p++] = r;
    raw[p++] = g;
    raw[p++] = b;
    raw[p++] = a;
  }
}

// --- minimal PNG encoder ---
const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
const png = Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const outDir = dirname(fileURLToPath(import.meta.url));
const out = join(outDir, "app-icon.png");
writeFileSync(out, png);
console.log(`Wrote ${out} (${SIZE}x${SIZE}, ${png.length} bytes)`);
