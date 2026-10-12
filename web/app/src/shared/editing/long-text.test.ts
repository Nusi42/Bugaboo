import { describe, expect, it } from "vitest";

import { heldBy, settle, world } from "./long-text-testing.ts";

describe("iss_17 the lock starts on the first key, not before", () => {
  it("looks at the text without taking a lock, and takes it on the first change", async () => {
    const w = world();
    w.editor.open();
    await settle();
    expect(w.editor.snapshot().phase.kind).toBe("open");
    expect(w.log).not.toContain("take e1");
    w.editor.type("Old words!");
    await settle();
    expect(w.log).toEqual(["take e1", "start-beats"]);
    expect(w.editor.snapshot().phase.kind).toBe("locked");
    expect(w.kept).toEqual([{ text: "Old words!", base: 3 }]);
  });

  it("sends the text so far with every heartbeat", async () => {
    const w = world();
    w.editor.open();
    w.editor.type("New words");
    await settle();
    await w.beat();
    expect(w.beats).toEqual([{ text: "New words", base: 3 }]);
  });

  it("does nothing for text that is not different from the saved text", async () => {
    const w = world();
    w.editor.open();
    w.editor.type("Old words");
    await settle();
    expect(w.editor.snapshot().phase.kind).toBe("open");
  });
});

describe("iss_17 saving and what follows", () => {
  it("saves with the token and the version, then reads the record again", async () => {
    const w = world();
    w.editor.open();
    w.editor.type("New words");
    await settle();
    await w.editor.save();
    expect(w.saves).toEqual([{ text: "New words", version: 3, lockToken: "t1" }]);
    expect(w.editor.snapshot().phase.kind).toBe("reading");
    expect(w.log).toContain("forget");
    expect(w.log.at(-1)).toBe("refresh");
    expect(w.log).not.toContain("release t1");
  });

  it("saves on the version now when only another field changed meanwhile", async () => {
    const w = world();
    w.editor.open();
    w.editor.type("New words");
    await settle();
    w.editor.setSource({ saved: "Old words", version: 5 });
    await w.editor.save();
    expect(w.saves[0]?.version).toBe(5);
  });

  it("keeps the base version when the text itself changed meanwhile", async () => {
    const w = world();
    w.editor.open();
    w.editor.type("New words");
    await settle();
    w.editor.setSource({ saved: "Ben's words", version: 5 });
    await w.editor.save();
    expect(w.saves[0]?.version).toBe(3);
  });
});

describe("iss_17 the editing ends", () => {
  it("throws the editing away with Cancel: the lock is let go and no draft is kept", async () => {
    const w = world();
    w.editor.open();
    w.editor.type("New words");
    await settle();
    w.editor.cancel();
    await settle();
    expect(w.log).toEqual(
      expect.arrayContaining(["stop-beats", "forget", "throw-away", "release t1"]),
    );
    expect(w.editor.snapshot().text).toBe("Old words");
  });
});

describe("iss_17 the editing ends", () => {
  it("lets go of the lock and the beats when the screen goes away, and keeps the draft", async () => {
    const w = world();
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    w.editor.dispose();
    await settle();
    expect(w.log).toEqual(expect.arrayContaining(["stop-beats", "release t1"]));
    expect(w.log).not.toContain("throw-away");
  });
});

describe("iss_17 somebody else is editing", () => {
  it("shows who, keeps the typed text, and takes over with Edit anyway", async () => {
    const w = world({ take: () => Promise.resolve(heldBy(true)) });
    w.editor.open();
    w.editor.type("My words");
    await settle();
    expect(w.editor.snapshot().phase.kind).toBe("held");
    expect(w.editor.snapshot().text).toBe("My words");
    await w.editor.askToFinish();
    expect(w.log).toContain("ask");
    await w.editor.editAnyway();
    expect(w.editor.snapshot().phase.kind).toBe("locked");
    expect(w.log).toContain("edit-anyway");
    expect(w.editor.snapshot().text).toBe("My words");
  });
});
