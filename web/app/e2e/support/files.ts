import { crc32, deflateSync } from "node:zlib";

import type { Locator, Page, Request } from "@playwright/test";

/** A file of the tests: what a person has on their device, with the type the device says. */
export interface Pictured {
  readonly name: string;
  readonly mimeType: string;
  readonly buffer: Buffer;
}

/** The size of one part of an upload, in bytes (`PART_BYTES` of the server, said by every start). */
export const PART_BYTES = 4 * 1024 * 1024;

const MEGABYTE = 1024 * 1024;

/** The first bytes of every PNG. */
const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** One chunk of a PNG: length, name, data, and the CRC-32 of name and data. */
function chunk(name: string, data: Buffer): Buffer {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const named = Buffer.concat([Buffer.from(name, "latin1"), data]);
  const check = Buffer.alloc(4);
  check.writeUInt32BE(crc32(named));
  return Buffer.concat([length, named, check]);
}

/** The colour of a pixel of the picture: a sky that fades into the ground, with a light window. */
function pixel(x: number, y: number, width: number, height: number): [number, number, number] {
  const inWindow = x > width * 0.3 && x < width * 0.7 && y > height * 0.25 && y < height * 0.6;
  if (inWindow) {
    return [250, 245, 230];
  }
  const fade = y / height;
  return [Math.round(40 + 190 * fade), Math.round(90 + 90 * fade), Math.round(200 - 140 * fade)];
}

/** A real PNG of `width` by `height` pixels, as a screenshot from the clipboard is. */
export function screenshot(name = "screenshot.png", width = 320, height = 200): Pictured {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  // 8 bits, RGB, no interlace.
  header.set([8, 2, 0, 0, 0], 8);
  const rows = Buffer.alloc((width * 3 + 1) * height);
  for (let y = 0; y < height; y += 1) {
    const start = y * (width * 3 + 1);
    // Each row begins with its filter, none.
    rows[start] = 0;
    for (let x = 0; x < width; x += 1) {
      rows.set(pixel(x, y, width, height), start + 1 + x * 3);
    }
  }
  return {
    name,
    mimeType: "image/png",
    buffer: Buffer.concat([
      PNG_SIGNATURE,
      chunk("IHDR", header),
      chunk("IDAT", deflateSync(rows)),
      chunk("IEND", Buffer.alloc(0)),
    ]),
  };
}

/** A real PDF of one page with a heading and a bar, as a report from a printer is. */
export function report(name = "quarter-report.pdf"): Pictured {
  const page = "BT /F1 28 Tf 30 130 Td (Quarterly report) Tj ET 0.2 0.4 0.8 rg 30 40 240 40 re f";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 180] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    `<< /Length ${String(page.length)} >>\nstream\n${page}\nendstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  ];
  const body = objects.map((text, at) => `${String(at + 1)} 0 obj\n${text}\nendobj\n`).join("");
  const text = `%PDF-1.4\n${body}trailer\n<< /Root 1 0 R /Size 6 >>\n%%EOF\n`;
  return { name, mimeType: "application/pdf", buffer: Buffer.from(text, "latin1") };
}

/** A file of plain bytes of this size, as a log or a video is. */
export function bytesOf(name: string, size: number): Pictured {
  return { name, mimeType: "application/octet-stream", buffer: Buffer.alloc(size, 0x61) };
}

/** A file of this many megabytes. */
export function megabytes(name: string, count: number): Pictured {
  return bytesOf(name, count * MEGABYTE);
}

/** What the page needs to make the file again: the bytes travel as text. */
interface Carried {
  readonly name: string;
  readonly type: string;
  readonly base64: string;
}

function carried(file: Pictured): Carried {
  return { name: file.name, type: file.mimeType, base64: file.buffer.toString("base64") };
}

/**
 * Pastes the file as a screenshot from the clipboard is pasted: a paste event with the file in
 * its data, on the element that has the focus (the page, when none has).
 */
export async function pasteFile(page: Page, file: Pictured): Promise<void> {
  await page.evaluate((sent) => {
    const bytes = Uint8Array.from(atob(sent.base64), (letter) => Number(letter.codePointAt(0)));
    const data = new DataTransfer();
    data.items.add(new File([bytes], sent.name, { type: sent.type }));
    const target = document.activeElement ?? document.body;
    target.dispatchEvent(
      new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }),
    );
  }, carried(file));
}

/** Drops the file on the place, the way a person drags it there: the three events a drag makes. */
export async function dropFile(place: Locator, file: Pictured): Promise<void> {
  await place.evaluate((element, sent) => {
    const bytes = Uint8Array.from(atob(sent.base64), (letter) => Number(letter.codePointAt(0)));
    const data = new DataTransfer();
    data.items.add(new File([bytes], sent.name, { type: sent.type }));
    // A file dragged from the desktop has an entry of the file system, which is how a place that
    // takes dropped files tells a file from a folder; one made by a script has none, so every
    // file of the page is given the entry of a file, the one thing the place asks of it.
    Object.defineProperty(DataTransferItem.prototype, "webkitGetAsEntry", {
      configurable: true,
      value(this: DataTransferItem) {
        return { isFile: true, isDirectory: false, name: this.getAsFile()?.name };
      },
    });
    // What a browser sets while a file is dragged over a place that accepts it.
    data.effectAllowed = "copy";
    data.dropEffect = "copy";
    for (const name of ["dragenter", "dragover", "drop"]) {
      element.dispatchEvent(
        new DragEvent(name, { dataTransfer: data, bubbles: true, cancelable: true }),
      );
    }
  }, carried(file));
}

/** Whether a value is the answer of `getFileRules`, as far as the tests read it. */
function isRules(value: unknown): value is { readonly limit_bytes: number } {
  return (
    typeof value === "object" &&
    value !== null &&
    "limit_bytes" in value &&
    typeof value.limit_bytes === "number"
  );
}

/** What the project of the issue allows for one file, as the page itself asks the server. */
export async function limitOf(page: Page, issue: string): Promise<number> {
  const rules: unknown = await (
    await page.request.get(`/api/v1/issues/${issue}/file-rules`)
  ).json();
  if (!isRules(rules)) {
    throw new TypeError("the rules of the files do not say the limit");
  }
  return rules.limit_bytes;
}

/** A limit in bytes as the page says it: whole megabytes (`25 MB`). */
export function limitInWords(limit: number): string {
  return `${String(limit / MEGABYTE)} MB`;
}

/** What a browser has sent to the server for the uploads of a page, and how many it started. */
export interface UploadsSeen {
  /** The offsets of the parts, in the order they were sent. */
  readonly offsets: number[];
  /** How many uploads were started. */
  started: number;
}

/** Watches the requests of the uploads of a page. */
export function watchUploads(page: Page): UploadsSeen {
  const seen: UploadsSeen = { offsets: [], started: 0 };
  page.on("request", (request: Readonly<Request>) => {
    const url = new URL(request.url());
    if (request.method() === "PUT" && url.pathname.startsWith("/api/v1/uploads/")) {
      seen.offsets.push(Number(url.searchParams.get("offset")));
    } else if (request.method() === "POST" && url.pathname.endsWith("/uploads")) {
      seen.started += 1;
    }
  });
  return seen;
}

/**
 * Answers the second part of an upload once as if the connection had broken (a request that timed
 * out). The part has reached the server (`arrives`) or has not.
 */
export async function breakSecondPartOnce(page: Page, arrives: boolean): Promise<void> {
  let broken = false;
  await page.route(/\/api\/v1\/uploads\/[^/?]+\?offset=/u, async (route) => {
    const offset = new URL(route.request().url()).searchParams.get("offset");
    if (route.request().method() === "PUT" && offset === String(PART_BYTES) && !broken) {
      broken = true;
      if (arrives) {
        await route.fetch();
      }
      await route.fulfill({ status: 408 });
      return;
    }
    await route.continue();
  });
}
