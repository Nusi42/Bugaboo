import { ApiError } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { course, delivery, done, fakeUploads, lostConnection, problem } from "./fake-uploads.ts";
import { extensionOf, formatSize } from "./size.ts";
import { sendFile, waitsBetweenTries } from "./transfer.ts";

const content = (): Blob => new Blob(["0123456789"]);

describe("att_01 an upload that resumes", () => {
  it("sends the file in parts that each say where they start", async () => {
    const { transport, sent } = fakeUploads();
    const c = course();
    await expect(sendFile(transport, delivery, content(), c)).resolves.toBe(done);
    expect(sent).toEqual([0, 4, 8]);
    expect(c.seen).toEqual([0, 4, 8, 10]);
    expect(c.opened).toEqual(["u1"]);
  });

  it("goes on from where the server stands after a lost connection", async () => {
    const { transport, sent } = fakeUploads([undefined, lostConnection()]);
    const c = course();
    await sendFile(transport, delivery, content(), c);
    expect(sent).toEqual([0, 4, 8]);
    expect(c.waited).toEqual([waitsBetweenTries[0]]);
  });

  it("asks where the server stands after a part out of order, without waiting", async () => {
    const wrong = new ApiError(409, problem("file.wrong-part", 409));
    const { transport, sent } = fakeUploads([wrong]);
    const c = course();
    await sendFile(transport, delivery, content(), c);
    expect(sent).toEqual([0, 4, 8]);
    expect(c.waited).toEqual([]);
  });
});

describe("att_01 an upload that gives up", () => {
  it("gives up after the last wait", async () => {
    const lost = lostConnection();
    const { transport } = fakeUploads(waitsBetweenTries.map(() => lost).concat(lost));
    await expect(sendFile(transport, delivery, content(), course())).rejects.toBe(lost);
  });

  it("gives up after the last wait also when parts went through before", async () => {
    const lost = lostConnection();
    const { transport, sent } = fakeUploads([
      undefined,
      ...waitsBetweenTries.map(() => lost),
      lost,
    ]);
    const c = course();
    await expect(sendFile(transport, delivery, content(), c)).rejects.toBe(lost);
    // The waits are those of one run of failures, not of one run for every part before it.
    expect(c.waited).toEqual(waitsBetweenTries);
    expect(sent).toEqual([0]);
  });

  it("tells the person at once when the disk of the server is full", async () => {
    const full = new ApiError(507, problem("file.no-room", 507));
    const { transport } = fakeUploads([full]);
    const c = course();
    await expect(sendFile(transport, delivery, content(), c)).rejects.toBe(full);
    expect(c.waited).toEqual([]);
  });

  it("does not ask again after a refusal", async () => {
    const refusal = new ApiError(413, problem("file.too-big", 413));
    const { transport } = fakeUploads([refusal]);
    const c = course();
    await expect(sendFile(transport, delivery, content(), c)).rejects.toBe(refusal);
    expect(c.waited).toEqual([]);
  });
});

describe("att_01 an upload that is taken up again", () => {
  it("goes on with an upload that was started before, from where the server has it", async () => {
    const { transport, sent, started, held } = fakeUploads();
    // A first try reached the server with its first two parts and was lost after that.
    const first = await transport.start(delivery);
    await transport.send(first.id, 0, content().slice(0, 4));
    await transport.send(first.id, 4, content().slice(4, 8));
    sent.length = 0;
    const c = course();
    await expect(sendFile(transport, delivery, content(), c, first.id)).resolves.toBe(done);
    // Only what the server did not have is sent, in the upload it has, and no new one is started.
    expect(sent).toEqual([8]);
    expect(started()).toBe(1);
    expect(c.opened).toEqual([first.id]);
    expect(held.get(first.id)).toBe(10);
  });

  it("starts a new upload when the one to go on with is gone", async () => {
    const { transport, sent, started } = fakeUploads();
    const c = course();
    await expect(sendFile(transport, delivery, content(), c, "u-that-is-gone")).resolves.toBe(done);
    expect(started()).toBe(1);
    expect(sent).toEqual([0, 4, 8]);
    expect(c.opened).toEqual(["u1"]);
  });
});

describe("att_01 sizes and types in words", () => {
  it("writes sizes with a unit", () => {
    expect(formatSize(1)).toBe("1 byte");
    expect(formatSize(34)).toBe("34 bytes");
    expect(formatSize(1536)).toBe("1.5 kB");
    expect(formatSize(25 * 1024 * 1024)).toBe("25 MB");
  });

  it("takes the type from the end of the name", () => {
    expect(extensionOf("REPORT.pdf")).toBe("PDF");
    expect(extensionOf(".hidden")).toBe("");
    expect(extensionOf("none")).toBe("");
  });
});
