import { inflateSync } from "node:zlib";

/** The pixels of a picture: the width, and the color channels row by row. */
export interface Pixels {
  readonly width: number;
  readonly channels: number;
  readonly data: Uint8Array;
}

/** What a filter of a PNG row adds up to: the byte left of it, above it and above left of it. */
function unfiltered(filter: number, raw: number, left: number, up: number, upLeft: number): number {
  const nearest = (): number => {
    const estimate = left + up - upLeft;
    const [dLeft, dUp, dUpLeft] = [left, up, upLeft].map((near) => Math.abs(estimate - near));
    if ((dLeft ?? 0) <= (dUp ?? 0) && (dLeft ?? 0) <= (dUpLeft ?? 0)) {
      return left;
    }
    return (dUp ?? 0) <= (dUpLeft ?? 0) ? up : upLeft;
  };
  const added = [0, left, up, (left + up) >> 1, nearest()][filter] ?? 0;
  return (raw + added) & 255;
}

/** Reads a PNG of 8 bits a channel without interlacing (what a screenshot of the browser is). */
export function readPng(file: Buffer): Pixels {
  const width = file.readUInt32BE(16);
  const height = file.readUInt32BE(20);
  const channels = file[25] === 6 ? 4 : 3;
  const parts: Buffer[] = [];
  for (let at = 8; at < file.length;) {
    const size = file.readUInt32BE(at);
    if (file.toString("latin1", at + 4, at + 8) === "IDAT") {
      parts.push(file.subarray(at + 8, at + 8 + size));
    }
    at += size + 12;
  }
  const raw = inflateSync(Buffer.concat(parts));
  const stride = width * channels;
  const data = new Uint8Array(stride * height);
  for (let row = 0; row < height; row += 1) {
    const filter = raw[row * (stride + 1)] ?? 0;
    for (let at = 0; at < stride; at += 1) {
      const byte = raw[row * (stride + 1) + 1 + at] ?? 0;
      const left = at >= channels ? (data[row * stride + at - channels] ?? 0) : 0;
      const up = row > 0 ? (data[(row - 1) * stride + at] ?? 0) : 0;
      const upLeft =
        row > 0 && at >= channels ? (data[(row - 1) * stride + at - channels] ?? 0) : 0;
      data[row * stride + at] = unfiltered(filter, byte, left, up, upLeft);
    }
  }
  return { width, channels, data };
}

/** The most that one color channel differs between two pictures of the same size. */
export function mostLevelsBetween(first: Buffer, second: Buffer): number {
  const [one, two] = [readPng(first), readPng(second)];
  let most = 0;
  one.data.forEach((level, at) => {
    most = Math.max(most, Math.abs(level - (two.data[at] ?? 0)));
  });
  return most;
}
