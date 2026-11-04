// The PWA's PNG icons, drawn at build time so no binary is committed: the logo's
// indigo-to-sky gradient, full bleed so it masks well, with a dark diamond in the middle.

import { deflateSync } from 'node:zlib';

const CRC = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buf: Buffer): number {
  let c = 0xffffffff;
  for (const b of buf) c = CRC[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type: string, data: Buffer): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}

const FROM = [0x8b, 0x9c, 0xff];
const TO = [0x5c, 0xc8, 0xff];
const DARK = [0x0b, 0x0e, 0x14];

export function iconPng(size: number): Buffer {
  const rows: Buffer[] = [];
  const half = size / 2;
  // Inside the maskable safe zone (the middle 80%).
  const reach = size * 0.24;
  for (let y = 0; y < size; y++) {
    const row = Buffer.alloc(1 + size * 3);
    for (let x = 0; x < size; x++) {
      const t = (x + y) / (2 * size);
      const inDiamond = Math.abs(x + 0.5 - half) + Math.abs(y + 0.5 - half) <= reach;
      const rgb = inDiamond ? DARK : FROM.map((f, i) => Math.round(f + (TO[i] - f) * t));
      row.set(rgb, 1 + x * 3);
    }
    rows.push(row);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 2; // truecolour
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(Buffer.concat(rows))),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}
