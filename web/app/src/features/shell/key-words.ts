import { t } from "@bugaboo/messages";

import { partsOfChord } from "../../shared/shortcuts-engine.ts";
import { chordsOfSequence } from "../../shared/shortcuts.ts";

/** Whether this computer is a Mac (or an iPhone or iPad), where Ctrl is the key with the sign ⌘. */
export function onMac(): boolean {
  return typeof navigator !== "undefined" && /Mac|iPhone|iPad/u.test(navigator.platform);
}

/** The words for the keys that are not a letter or a sign; `mac` says which computer reads them. */
export function keyWord(part: string, mac: boolean = onMac()): string {
  switch (part) {
    case "Mod": {
      return mac ? t("shortcut.key.mod-mac") : t("shortcut.key.mod");
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
