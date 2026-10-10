import type { MessageKey } from "@bugaboo/messages";

/**
 * The keyboard shortcuts of Bugaboo, defined once (UX-05, UX §6.2): the ID, the default keys, the
 * words that name it, where it works, and the visible button that does the same (principle 9:
 * shortcuts are extras, never required). The list under `?`, the page in *My settings* and the
 * handlers all read from here.
 *
 * Keys are written as the chords that make them: `Mod+k` is Ctrl+K (Cmd+K on a Mac), and a
 * sequence is chords separated by a space: `g h` is G, then H.
 */
export const shortcutDefinitions = [
  {
    id: "quick-search",
    keys: ["Mod+k"],
    message: "shortcut.quick-search",
    where: "everywhere",
    button: "shell.search",
    inFields: true,
  },
  {
    id: "undo",
    keys: ["Mod+z"],
    message: "shortcut.undo",
    where: "everywhere",
    button: "common.undo",
  },
  {
    id: "show-shortcuts",
    keys: ["?"],
    message: "shortcut.show-shortcuts",
    where: "everywhere",
    button: "shortcut.title",
  },
  {
    id: "go-home",
    keys: ["g h"],
    message: "shortcut.go-home",
    where: "everywhere",
    button: "nav.home",
  },
  {
    id: "new-issue",
    keys: ["c"],
    message: "shortcut.new-issue",
    where: "everywhere",
    button: "issues.new",
  },
  {
    id: "list-next",
    keys: ["j"],
    message: "shortcut.list-next",
    where: "lists",
    button: "list.next",
  },
  {
    id: "list-previous",
    keys: ["k"],
    message: "shortcut.list-previous",
    where: "lists",
    button: "list.previous",
  },
  {
    id: "change-status",
    keys: ["s"],
    message: "shortcut.change-status",
    where: "issues",
    button: "issue-field.status",
  },
  {
    id: "assign",
    keys: ["a"],
    message: "shortcut.assign",
    where: "issues",
    button: "issue-field.assigned-to",
  },
  {
    id: "edit-description",
    keys: ["e"],
    message: "shortcut.edit-description",
    where: "issues",
    button: "issues.description.edit",
  },
] as const satisfies readonly ShortcutDefinition[];

/** Where a shortcut works. */
export type ShortcutPlace = "everywhere" | "lists" | "issues";

/** The words that say where a shortcut works, for the list under `?` and the page in *My settings*. */
export const placeMessage = {
  everywhere: "shortcut.everywhere",
  lists: "shortcut.in-lists",
  issues: "shortcut.on-issue",
} as const satisfies Record<ShortcutPlace, MessageKey>;

export interface ShortcutDefinition {
  readonly id: string;
  /** The default keys; any of them does the action. */
  readonly keys: readonly string[];
  /** What the shortcut does, in words. */
  readonly message: MessageKey;
  /** Where it works: on every screen, only where a list is shown, or only on an open issue. */
  readonly where: ShortcutPlace;
  /** The name of the visible button or link that does the same. */
  readonly button: MessageKey;
  /**
   * Works while a text field has the focus too (a key with Ctrl or Cmd cannot be typed text):
   * quick search is wanted from anywhere. Without it the shortcut stays silent in fields.
   */
  readonly inFields?: boolean;
}

export type ShortcutId = (typeof shortcutDefinitions)[number]["id"];

/** The keys a person chose to use instead of the defaults, by shortcut. */
export type ShortcutOverrides = Readonly<Partial<Record<ShortcutId, readonly string[]>>>;

/** The definition of a shortcut. */
export function definitionOf(id: ShortcutId): ShortcutDefinition {
  const found = shortcutDefinitions.find((definition) => definition.id === id);
  // The type of `id` says it is in the list; the fallback is only for the compiler.
  return found ?? shortcutDefinitions[0];
}

/** The keys that work for a shortcut: the person's own, else the defaults. */
export function keysOf(id: ShortcutId, overrides: ShortcutOverrides): readonly string[] {
  return overrides[id] ?? definitionOf(id).keys;
}

/** The chords of a key sequence: `g h` is `["g", "h"]`. */
export function chordsOfSequence(sequence: string): readonly string[] {
  return sequence.split(" ").filter((chord) => chord !== "");
}

/** A chord is a key, optionally with `Mod+`, `Alt+` and `Shift+` in front, and never empty. */
const chordPattern = /^(?:(?:Mod|Alt|Shift)\+)*(?:[^\s+]|[A-Za-z][A-Za-z0-9]+)$/u;

/** Whether text is a valid key sequence of one or two chords. */
export function isValidSequence(sequence: string): boolean {
  const chords = chordsOfSequence(sequence);
  return (
    chords.length > 0 && chords.length <= 2 && chords.every((chord) => chordPattern.test(chord))
  );
}

/**
 * Keys that belong to the browser and to every text field: select all, copy, paste, cut, undo and
 * redo. A shortcut of Bugaboo may not take them (the usual ones, like Undo, are the registry's own).
 */
const reservedChords: ReadonlySet<string> = new Set([
  "Mod+a",
  "Mod+c",
  "Mod+v",
  "Mod+x",
  "Mod+z",
  "Mod+y",
  "Mod+Shift+z",
]);

/** Whether these keys are one of the browser's own (copy, paste, undo ...). */
export function isReservedKeys(sequence: string): boolean {
  return chordsOfSequence(sequence).some((chord) => reservedChords.has(chord));
}

/** What a key sequence collides with, if anything. */
export interface Conflict {
  readonly id: ShortcutId;
  readonly keys: string;
}

/** Whether one sequence is the other, or the start of it: then the shorter would hide the longer. */
function overlaps(first: string, second: string): boolean {
  const a = chordsOfSequence(first);
  const b = chordsOfSequence(second);
  const length = Math.min(a.length, b.length);
  return a.slice(0, length).join(" ") === b.slice(0, length).join(" ");
}

/** The other shortcut that already uses (the start of) these keys, so the choice can be refused. */
export function conflictFor(
  id: ShortcutId,
  keys: string,
  overrides: ShortcutOverrides,
): Conflict | null {
  for (const definition of shortcutDefinitions) {
    if (definition.id !== id) {
      const taken = keysOf(definition.id, overrides).find((other) => overlaps(other, keys));
      if (taken !== undefined) {
        return { id: definition.id, keys: taken };
      }
    }
  }
  return null;
}

/** The functions that do what each shortcut says; a shortcut without one does nothing here. */
export type ShortcutHandlers = Readonly<Partial<Record<ShortcutId, () => void>>>;

/** The shortcut with this ID, or `undefined` for text that is none (an address typed by hand). */
export function shortcutIdOf(value: unknown): ShortcutId | undefined {
  return shortcutDefinitions.find((definition) => definition.id === value)?.id;
}
