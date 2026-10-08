import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { partsOfChord } from "../../shared/shortcuts-engine.ts";
import { chordsOfSequence } from "../../shared/shortcuts.ts";
import { keyWord } from "./key-words.ts";

export interface KeysProps {
  /** The keys, written as in the registry: `Mod+k`, `g h`. */
  readonly sequence: string;
}

/**
 * The keys of a shortcut, drawn as keys. Screen readers read the same words ("Ctrl", "K", "then",
 * "H"), so nothing is said twice.
 */
export function Keys({ sequence }: KeysProps) {
  const chords = chordsOfSequence(sequence);
  return (
    <span {...partOf(part.keys)}>
      {chords.map((chord, index) => (
        <span key={chords.slice(0, index + 1).join(" ")} {...partOf(part.keysChord)}>
          {index === 0 ? null : <span {...partOf(part.keysThen)}>{t("shortcut.then")}</span>}
          {partsOfChord(chord).map((piece) => (
            <kbd key={piece} {...partOf(part.keysKey)}>
              {keyWord(piece)}
            </kbd>
          ))}
        </span>
      ))}
    </span>
  );
}
