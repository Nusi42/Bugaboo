import {
  chordsOfSequence,
  keysOf,
  shortcutDefinitions,
  type ShortcutHandlers,
  type ShortcutId,
  type ShortcutOverrides,
} from "./shortcuts.ts";

/** How long the second key of a sequence may take. */
export const sequenceMs = 1000;

/** The part of a keyboard event the shortcuts look at. */
export interface KeyEventLike {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
}

const modifierKeys = new Set(["Control", "Shift", "Alt", "Meta", "AltGraph", "CapsLock"]);

/** Whether this computer is a Mac (or an iPhone or iPad), where Cmd is the key for shortcuts. */
export function onMac(): boolean {
  return typeof navigator !== "undefined" && /Mac|iPhone|iPad/u.test(navigator.platform);
}

/**
 * The chord a key press makes, written as in the definitions (`Mod+k`, `?`, `Escape`), or `null`
 * for the press of a modifier alone.
 *
 * `Mod` is Cmd on a Mac and Ctrl everywhere else. The other one stays what it is (`Ctrl` on a Mac,
 * `Meta` elsewhere), so Ctrl+K on a Mac still deletes to the end of the line and matches nothing.
 * Shift counts for every key that is not a single character, and for every key pressed with `Mod`,
 * so Redo (Ctrl+Shift+Z) never fires Undo; the shift of `?` alone is part of the character. With
 * `Mod` a letter is the same key whatever the caps lock says.
 */
export function chordOf(event: KeyEventLike, mac: boolean = onMac()): string | null {
  if (modifierKeys.has(event.key)) {
    return null;
  }
  const single = event.key.length === 1;
  const mod = mac ? event.metaKey : event.ctrlKey;
  const parts: string[] = [];
  if (mod) {
    parts.push("Mod");
  }
  if (event.ctrlKey && !mod) {
    parts.push("Ctrl");
  }
  if (event.metaKey && !mod) {
    parts.push("Meta");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.shiftKey && (!single || mod)) {
    parts.push("Shift");
  }
  parts.push(single && mod ? event.key.toLowerCase() : event.key);
  return parts.join("+");
}

/** What a field with the focus looks like to the shortcuts. */
export interface FocusTargetLike {
  readonly tagName?: string;
  readonly type?: string;
  readonly isContentEditable?: boolean;
}

/** Inputs that take no typing: a shortcut may fire while one of them has the focus. */
const inputsWithoutText = new Set([
  "button",
  "checkbox",
  "color",
  "file",
  "image",
  "radio",
  "range",
  "reset",
  "submit",
]);

/** Whether the focus is in something the person types into: shortcuts then stay silent. */
export function isTyping(target: FocusTargetLike | null): boolean {
  if (target === null) {
    return false;
  }
  if (target.isContentEditable === true) {
    return true;
  }
  const tag = target.tagName?.toLowerCase();
  if (tag === "textarea" || tag === "select") {
    return true;
  }
  return tag === "input" && !inputsWithoutText.has((target.type ?? "text").toLowerCase());
}

export interface EngineOptions {
  readonly overrides: () => ShortcutOverrides;
  /** The handlers of the screen that is open now. */
  readonly handlers: () => ShortcutHandlers;
  readonly now: () => number;
  /** Whether this is a Mac; the computer that is running by default. */
  readonly mac?: () => boolean;
}

/** What the engine needs of a key press and where it happened. */
export interface PressLike extends KeyEventLike {
  readonly target: FocusTargetLike | null;
  readonly isComposing?: boolean;
}

interface Candidate {
  readonly id: ShortcutId;
  readonly chords: readonly string[];
}

/** Every key sequence that does something now, with the shortcut it belongs to. */
function candidatesFor(
  overrides: ShortcutOverrides,
  handlers: ShortcutHandlers,
  typing: boolean,
): readonly Candidate[] {
  const found: Candidate[] = [];
  for (const definition of shortcutDefinitions) {
    // While somebody types only the shortcuts made for it work, and only with Ctrl or Cmd.
    const allowed = !typing || ("inFields" in definition && definition.inFields);
    if (handlers[definition.id] !== undefined && allowed) {
      for (const keys of keysOf(definition.id, overrides)) {
        const chords = chordsOfSequence(keys);
        if (!typing || chords.every((chord) => chord.startsWith("Mod+"))) {
          found.push({ id: definition.id, chords });
        }
      }
    }
  }
  return found;
}

/**
 * Turns key presses into shortcuts. It never fires while somebody types in a field, understands
 * sequences such as `g h` (the second key has `sequenceMs` to come), and uses the person's own
 * keys where they replaced the defaults. Returns whether the press was a shortcut.
 */
export function createEngine(options: EngineOptions): (press: PressLike) => boolean {
  let pending: readonly string[] = [];
  let pendingSince = 0;
  return (press) => {
    const chord = chordOf(press, options.mac?.() ?? onMac());
    if (chord === null) {
      return false;
    }
    if (press.isComposing === true) {
      pending = [];
      return false;
    }
    const typing = isTyping(press.target);
    if (typing) {
      pending = [];
    }
    const now = options.now();
    const before = now - pendingSince > sequenceMs ? [] : pending;
    const handlers = options.handlers();
    const candidates = candidatesFor(options.overrides(), handlers, typing);
    // The keys so far, and then the press alone: a wrong second key may start a new sequence.
    for (const typed of [[...before, chord], [chord]]) {
      const done = candidates.find((candidate) => candidate.chords.join(" ") === typed.join(" "));
      if (done !== undefined) {
        pending = [];
        handlers[done.id]?.();
        return true;
      }
      const starts = candidates.some(
        (candidate) =>
          candidate.chords.length > typed.length &&
          typed.every((one, index) => candidate.chords[index] === one),
      );
      if (starts) {
        pending = typed;
        pendingSince = now;
        return true;
      }
    }
    pending = [];
    return false;
  };
}

/** The keys of a chord for people to read: `Mod+k` is `["Mod", "k"]`; the caller adds the words. */
export function partsOfChord(chord: string): readonly string[] {
  const plus = chord.endsWith("++") ? chord.length - 2 : chord.lastIndexOf("+");
  if (plus <= 0) {
    return [chord];
  }
  return [...chord.slice(0, plus).split("+"), chord.slice(plus + 1)];
}
