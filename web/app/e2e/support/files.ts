import { crc32, deflateSync } from "node:zlib";

import { megabyte, partBytes } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page, type Request } from "@playwright/test";

import { nothing, onThisLayout } from "./layout.ts";
import { partIn } from "./parts.ts";
import { boxOf, textStartOf } from "./windows.ts";

/** How long a preview may take: the job runs every two seconds, and a picture is made at once. */
export const PREVIEW_WAIT_MS = 30_000;

/** The block of the files of the issue that is open. */
export function filesBlock(page: Page): Locator {
  return page.getByRole("region", { name: t("files.title"), exact: true });
}

/** The row of a file by its name. */
export function rowOf(page: Page, name: string): Locator {
  return filesBlock(page).getByRole("listitem").filter({ hasText: name });
}

/** The small picture in the row of a file, once its preview is made. */
export function previewOf(page: Page, name: string): Locator {
  return rowOf(page, name).getByRole("img", { name: t("files.preview-of", { name }) });
}

/** A file of the tests: what a person has on their device, with the type the device says. */
export interface Pictured {
  readonly name: string;
  readonly mimeType: string;
  readonly buffer: Buffer;
}

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
  return bytesOf(name, count * megabyte);
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
 * Pastes the file as a screenshot from the clipboard is pasted, on the element that has the focus.
 * The page takes a paste only once its place to drop files is there, so this waits for it.
 */
export async function pasteFile(page: Page, file: Pictured): Promise<void> {
  const place = filesBlock(page).getByRole("button", { name: t("files.drop-place"), exact: true });
  await expect(place).toBeVisible();
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

/** A file one megabyte over the limit (in bytes) of the project. */
export const overTheLimit = (name: string, limit: number): Pictured =>
  megabytes(name, limit / megabyte + 1);

/** A limit in bytes as the page says it: whole megabytes (`25 MB`). */
export function limitInWords(limit: number): string {
  return `${String(limit / megabyte)} MB`;
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
    if (route.request().method() === "PUT" && offset === String(partBytes) && !broken) {
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

/**
 * Chooses files with the button of a place (*Upload*), as a person does with the window of their
 * device.
 */
export async function chooseFiles(
  page: Page,
  button: Locator,
  files: readonly Pictured[],
): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await button.click();
  await (await chooser).setFiles([...files]);
}

/** What lets go of a part that is being held. */
export interface Hold {
  readonly release: () => void;
}

/**
 * Keeps the second part of an upload from reaching the server until `release` is called, so the
 * upload stands still a part of the way and the bar of it can be looked at.
 */
export async function holdSecondPart(page: Page): Promise<Hold> {
  const { promise: released, resolve: release } = Promise.withResolvers<void>();
  await page.route(/\/api\/v1\/uploads\/[^/?]+\?offset=/u, async (route) => {
    const offset = new URL(route.request().url()).searchParams.get("offset");
    if (route.request().method() === "PUT" && offset === String(partBytes)) {
      await released;
    }
    await route.continue();
  });
  return { release };
}

/** The border of a quiet button, transparent: the text of every button that hangs back is this far in. */
const QUIET_BORDER_PX = 1;

/**
 * On a phone, *Download* sits under the name and its text starts on the edge of the tile above
 * (R1). On a computer the buttons are beside the name and are on the edge of nothing.
 */
export async function expectActionsOnTileEdge(
  page: Page,
  row: Locator,
  name: string,
): Promise<void> {
  await onThisLayout(page, {
    computer: nothing,
    tablet: nothing,
    phone: async () => {
      const tile = await boxOf(partIn(row, "files.tile"));
      const download = row.getByRole("button", { name: t("files.download-named", { name }) });
      // The text of a button starts after its own padding: that is the edge that has to line up.
      const textStart = await textStartOf(download);
      expect(Math.abs(textStart - tile.x)).toBeLessThanOrEqual(QUIET_BORDER_PX);
    },
  });
}
