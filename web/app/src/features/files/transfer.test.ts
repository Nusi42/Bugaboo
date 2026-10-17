import { ApiError, type ErrorCode, type Problem } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  sendFile,
  waitsBetweenTries,
  type Attachment,
  type Course,
  type Delivery,
  type Transport,
  type UploadState,
} from "./transfer.ts";
import { extensionOf, formatSize } from "./size.ts";

const delivery: Delivery = { issue: "WEB-1", comment: null, name: "a.txt", size: 10 };
const done: Attachment = {
  added_at: "2026-01-01T00:00:00Z",
  added_by: { shown: "someone" },
  download_url: "/files/x",
  id: "att",
  issue: "WEB-1",
  may_delete: true,
  media_type: "text/plain",
  name: "a.txt",
  size: 10,
  version: 1,
};
function problem(code: ErrorCode, status: number): Problem {
  return {
    type: code,
    title: "x",
    status,
    code,
    message_key: "error.server.unavailable",
    params: {},
  };
}
const lost = new ApiError(0, undefined);

function course(): Course & { readonly waited: number[]; readonly seen: number[] } {
  const waited: number[] = [];
  const seen: number[] = [];
  return {
    waited,
    seen,
    wait: (ms) => {
      waited.push(ms);
      return Promise.resolve();
    },
    progress: (sent) => {
      seen.push(sent);
    },
  };
}

/** A server that keeps `have` bytes and takes parts of four bytes; `failures` break sends. */
function server(failures: readonly Readonly<ApiError>[]) {
  const state = { have: 0 };
  const sent: number[] = [];
  let tries = 0;
  const upload = (): UploadState => ({ id: "u1", offset: state.have, part_bytes: 4, size: 10 });
  const transport: Transport = {
    start: () => Promise.resolve(upload()),
    state: () => Promise.resolve(upload()),
    send: (_id, offset, bytes) => {
      const failure = failures[tries];
      tries += 1;
      if (failure !== undefined) {
        return Promise.reject(failure);
      }
      sent.push(offset);
      state.have = offset + bytes.size;
      return Promise.resolve(upload());
    },
    finish: () => Promise.resolve(done),
  };
  return { transport, sent };
}

describe("att_01 an upload that resumes", () => {
  it("sends the file in parts that each say where they start", async () => {
    const { transport, sent } = server([]);
    const c = course();
    await expect(sendFile(transport, delivery, new Blob(["0123456789"]), c)).resolves.toBe(done);
    expect(sent).toEqual([0, 4, 8]);
    expect(c.seen).toEqual([0, 4, 8, 10]);
  });

  it("goes on from where the server stands after a lost connection", async () => {
    const { transport, sent } = server([lost]);
    const c = course();
    await sendFile(transport, delivery, new Blob(["0123456789"]), c);
    expect(sent).toEqual([0, 4, 8]);
    expect(c.waited).toEqual([waitsBetweenTries[0]]);
  });

  it("asks where the server stands after a part out of order, without waiting", async () => {
    const wrong = new ApiError(409, problem("file.wrong_part", 409));
    const { transport, sent } = server([wrong]);
    const c = course();
    await sendFile(transport, delivery, new Blob(["0123456789"]), c);
    expect(sent).toEqual([0, 4, 8]);
    expect(c.waited).toEqual([]);
  });

  it("gives up after the last wait", async () => {
    const { transport } = server(waitsBetweenTries.map(() => lost).concat(lost));
    await expect(sendFile(transport, delivery, new Blob(["0123456789"]), course())).rejects.toBe(
      lost,
    );
  });

  it("tells the person at once when the disk of the server is full", async () => {
    const full = new ApiError(507, problem("file.no_room", 507));
    const { transport } = server([full]);
    const c = course();
    await expect(sendFile(transport, delivery, new Blob(["0123456789"]), c)).rejects.toBe(full);
    expect(c.waited).toEqual([]);
  });

  it("does not ask again after a refusal", async () => {
    const refusal = new ApiError(413, problem("file.too_big", 413));
    const { transport } = server([refusal]);
    const c = course();
    await expect(sendFile(transport, delivery, new Blob(["0123456789"]), c)).rejects.toBe(refusal);
    expect(c.waited).toEqual([]);
  });
});

describe("att_01 sizes and types in words", () => {
  it("writes sizes with a unit", () => {
    expect(formatSize(312)).toBe("312 byte");
    expect(formatSize(1536)).toBe("1.5 kB");
    expect(formatSize(25 * 1024 * 1024)).toBe("25 MB");
  });

  it("takes the type from the end of the name", () => {
    expect(extensionOf("REPORT.pdf")).toBe("PDF");
    expect(extensionOf(".hidden")).toBe("");
    expect(extensionOf("none")).toBe("");
  });
});
