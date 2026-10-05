import { t } from "@bugaboo/messages";

import { partsOfChord } from "../../shared/shortcuts-engine.ts";
import { chordsOfSequence } from "../../shared/shortcuts.ts";
import { keyWord } from "./key-words.ts";
import styles from "./Keys.module.css";

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
    <span className={styles.keys}>
      {chords.map((chord, index) => (
        <span key={chords.slice(0, index + 1).join(" ")} className={styles.chord}>
          {index === 0 ? null : <span className={styles.then}>{t("shortcut.then")}</span>}
          {partsOfChord(chord).map((part) => (
            <kbd key={part} className={styles.key}>
              {keyWord(part)}
            </kbd>
          ))}
        </span>
      ))}
    </span>
  );
}
