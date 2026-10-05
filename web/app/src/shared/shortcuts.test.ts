import { isMessageKey } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import {
  chordOf,
  createEngine,
  isTyping,
  partsOfChord,
  sequenceMs,
  type FocusTargetLike,
  type PressLike,
} from "./shortcuts-engine.ts";
import {
  conflictFor,
  isValidSequence,
  keysOf,
  parseOverrides,
  shortcutDefinitions,
  type ShortcutId,
  type ShortcutOverrides,
} from "./shortcuts.ts";

/** A key press on the page, outside any field. */
function press(key: string, extra: Partial<PressLike> = {}): PressLike {
  return {
    key,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    target: { tagName: "BODY" },
    ...extra,
  };
}

/** An engine whose handlers write what they did into `log`. */
function engineWith(overrides: ShortcutOverrides = {}) {
  const log: ShortcutId[] = [];
  let time = 0;
  const handlers = Object.fromEntries(
    shortcutDefinitions.map((definition) => [
      definition.id,
      () => {
        log.push(definition.id);
      },
    ]),
  );
  const handle = createEngine({
    overrides: () => overrides,
    handlers: () => handlers,
    now: () => time,
  });
  return {
    log,
    handle,
    wait: (ms: number) => {
      time += ms;
    },
  };
}

describe("ux_05 the registry of shortcuts", () => {
  it("names every shortcut once, in words, with a visible button that does the same", () => {
    const ids = shortcutDefinitions.map((definition) => definition.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const definition of shortcutDefinitions) {
      expect(isMessageKey(definition.message)).toBe(true);
      expect(isMessageKey(definition.button)).toBe(true);
      expect(definition.keys.length).toBeGreaterThan(0);
      expect(definition.keys.every((keys) => isValidSequence(keys))).toBe(true);
    }
  });

  it("gives no two shortcuts the same keys, or keys that start one another", () => {
    for (const definition of shortcutDefinitions) {
      for (const keys of definition.keys) {
        expect(conflictFor(definition.id, keys, {})).toBeNull();
      }
    }
  });
});

describe("ux_05 pressing a shortcut", () => {
  it("does what the shortcut says, with Ctrl or Cmd for Mod", () => {
    const { log, handle } = engineWith();
    expect(handle(press("k", { ctrlKey: true }))).toBe(true);
    expect(handle(press("K", { metaKey: true, shiftKey: true }))).toBe(true);
    expect(handle(press("?", { shiftKey: true }))).toBe(true);
    expect(log).toEqual(["quick-search", "quick-search", "show-shortcuts"]);
  });

  it("understands sequences such as g h, and lets the second key take its time", () => {
    const { log, handle, wait } = engineWith();
    expect(handle(press("g"))).toBe(true);
    expect(log).toEqual([]);
    expect(handle(press("h"))).toBe(true);
    expect(log).toEqual(["go-home"]);
    handle(press("g"));
    wait(sequenceMs + 1);
    expect(handle(press("h"))).toBe(false);
    expect(log).toEqual(["go-home"]);
  });

  it("leaves the slash for the search bar of a later version (UX 6.2)", () => {
    const { log, handle } = engineWith();
    expect(handle(press("/"))).toBe(false);
    expect(log).toEqual([]);
  });

  it("does nothing for a key that is not a shortcut, and for a modifier alone", () => {
    const { log, handle } = engineWith();
    expect(handle(press("x"))).toBe(false);
    expect(handle(press("Control", { ctrlKey: true }))).toBe(false);
    expect(handle(press("k"))).toBe(true);
    expect(log).toEqual(["list-previous"]);
  });

  it("does nothing for a shortcut that has no handler on this screen", () => {
    const handle = createEngine({ overrides: () => ({}), handlers: () => ({}), now: () => 0 });
    expect(handle(press("k", { ctrlKey: true }))).toBe(false);
  });
});

describe("ux_05 a shortcut never fires while somebody types", () => {
  const fields: readonly FocusTargetLike[] = [
    { tagName: "INPUT", type: "text" },
    { tagName: "INPUT", type: "password" },
    { tagName: "INPUT" },
    { tagName: "TEXTAREA" },
    { tagName: "SELECT" },
    { tagName: "DIV", isContentEditable: true },
  ];

  it.each(fields)("is silent in %o", (target) => {
    const { log, handle } = engineWith();
    expect(handle(press("z", { ctrlKey: true, target }))).toBe(false);
    expect(handle(press("?", { target, shiftKey: true }))).toBe(false);
    expect(handle(press("j", { target }))).toBe(false);
    expect(log).toEqual([]);
  });

  it.each(fields)("except quick search, which is wanted from anywhere: in %o", (target) => {
    const { log, handle } = engineWith();
    expect(handle(press("k", { ctrlKey: true, target }))).toBe(true);
    expect(handle(press("k", { metaKey: true, target }))).toBe(true);
    expect(log).toEqual(["quick-search", "quick-search"]);
  });

  it("never lets a key with no Ctrl or Cmd through, whatever the shortcut says", () => {
    const { log, handle } = engineWith({ "quick-search": ["Alt+s"] });
    expect(handle(press("s", { altKey: true, target: { tagName: "INPUT", type: "text" } }))).toBe(
      false,
    );
    expect(log).toEqual([]);
  });

  it("forgets a sequence that was started before the focus moved into a field", () => {
    const { log, handle } = engineWith();
    handle(press("g"));
    handle(press("x", { target: { tagName: "INPUT", type: "text" } }));
    handle(press("h"));
    expect(log).toEqual([]);
  });

  it("still works when a button, a checkbox or a radio button has the focus", () => {
    expect(isTyping({ tagName: "BUTTON" })).toBe(false);
    expect(isTyping({ tagName: "INPUT", type: "checkbox" })).toBe(false);
    expect(isTyping({ tagName: "INPUT", type: "radio" })).toBe(false);
    expect(isTyping(null)).toBe(false);
  });
});

describe("ux_05 a person's own keys", () => {
  const own: ShortcutOverrides = { "quick-search": ["Alt+s"] };

  it("replace the defaults: the new key works and the old one does not", () => {
    const { log, handle } = engineWith(own);
    expect(handle(press("s", { altKey: true }))).toBe(true);
    expect(handle(press("k", { ctrlKey: true }))).toBe(false);
    expect(log).toEqual(["quick-search"]);
    expect(keysOf("quick-search", own)).toEqual(["Alt+s"]);
    expect(keysOf("go-home", own)).toEqual(["g h"]);
  });

  it("are refused when another shortcut uses them or their start", () => {
    expect(conflictFor("quick-search", "g h", {})?.id).toBe("go-home");
    expect(conflictFor("quick-search", "g", {})?.id).toBe("go-home");
    expect(conflictFor("quick-search", "j", {})?.id).toBe("list-next");
    expect(conflictFor("quick-search", "Alt+s", {})).toBeNull();
  });

  it("are read back from the saved text, leaving out what is not valid", () => {
    expect(parseOverrides(null)).toEqual({});
    expect(parseOverrides("nonsense")).toEqual({});
    expect(
      parseOverrides(
        JSON.stringify({ "quick-search": ["Alt+s"], "go-home": ["a b c"], nothing: ["x"] }),
      ),
    ).toEqual({ "quick-search": ["Alt+s"] });
  });
});

describe("ux_05 how keys are written", () => {
  it("reads the chord of a press", () => {
    expect(chordOf(press("k", { ctrlKey: true }))).toBe("Mod+k");
    expect(chordOf(press("Tab", { shiftKey: true }))).toBe("Shift+Tab");
    expect(chordOf(press("?", { shiftKey: true }))).toBe("?");
    expect(chordOf(press("Shift", { shiftKey: true }))).toBeNull();
  });

  it("splits a chord into the keys people press", () => {
    expect(partsOfChord("Mod+k")).toEqual(["Mod", "k"]);
    expect(partsOfChord("Mod++")).toEqual(["Mod", "+"]);
    expect(partsOfChord("?")).toEqual(["?"]);
  });
});
