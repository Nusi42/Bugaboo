import { t } from "@bugaboo/messages";

import { partsOfChord } from "../../shared/shortcuts-engine.ts";
import { chordsOfSequence } from "../../shared/shortcuts.ts";

/** The words for the keys that are not a letter or a sign. */
export function keyWord(part: string): string {
  switch (part) {
    case "Mod": {
      return t("shortcut.key.mod");
    }
    case "Alt": {
      return t("shortcut.key.alt");
    }
    case "Shift": {
      return t("shortcut.key.shift");
    }
    case "Escape": {
      return t("shortcut.key.escape");
    }
    case "Enter": {
      return t("shortcut.key.enter");
    }
    default: {
      return part.length === 1 ? part.toUpperCase() : part;
    }
  }
}

/** A key sequence as people say it: "Ctrl K", "G then H". */
export function describeKeys(sequence: string): string {
  return chordsOfSequence(sequence)
    .map((chord) =>
      partsOfChord(chord)
        .map((part) => keyWord(part))
        .join(" "),
    )
    .join(` ${t("shortcut.then")} `);
}
