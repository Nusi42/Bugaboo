import { inflateRawSync } from "node:zlib";

// Reads one file out of a ZIP file, as a person opens the file of *Download my data* (PRIV-03): the
// list at the end of the file names each part, where it starts and how it is packed.

/** The marks the parts of a ZIP file start with. */
const END_OF_LIST = 0x06_05_4b_50;
const LISTED = 0x02_01_4b_50;
const LOCAL = 0x04_03_4b_50;
/** How a part is packed: as it is, or with deflate. */
const STORED = 0;
const DEFLATED = 8;
/** The fixed sizes of the record at the end, of a listed part and of the head of a part. */
const END_SIZE = 22;
const LISTED_SIZE = 46;
const LOCAL_SIZE = 30;

/** The text of the part called `name` of a ZIP file. */
export function textIn(zip: Buffer, name: string): string {
  const end = zip.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  if (end < 0 || zip.readUInt32LE(end) !== END_OF_LIST || zip.length - end < END_SIZE) {
    throw new Error("The file is not a ZIP file.");
  }
  let at = zip.readUInt32LE(end + 16);
  for (let left = zip.readUInt16LE(end + 10); left > 0; left -= 1) {
    if (zip.readUInt32LE(at) !== LISTED) {
      throw new Error("The list of the ZIP file is damaged.");
    }
    const method = zip.readUInt16LE(at + 10);
    const packed = zip.readUInt32LE(at + 20);
    const nameLength = zip.readUInt16LE(at + 28);
    const listedName = zip.toString("utf8", at + LISTED_SIZE, at + LISTED_SIZE + nameLength);
    const local = zip.readUInt32LE(at + 42);
    if (listedName === name && zip.readUInt32LE(local) === LOCAL) {
      const start =
        local + LOCAL_SIZE + zip.readUInt16LE(local + 26) + zip.readUInt16LE(local + 28);
      const bytes = zip.subarray(start, start + packed);
      if (method === STORED) {
        return bytes.toString("utf8");
      }
      if (method === DEFLATED) {
        return inflateRawSync(bytes).toString("utf8");
      }
    }
    at += LISTED_SIZE + nameLength + zip.readUInt16LE(at + 30) + zip.readUInt16LE(at + 32);
  }
  throw new Error(`The ZIP file has no part ${name}.`);
}
