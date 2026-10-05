import { t } from "@bugaboo/messages";

import { partsOfChord } from "../../shared/shortcuts-engine.ts";
import { chordsOfSequence } from "../../shared/shortcuts.ts";
import { describeKeys, keyWord } from "./key-words.ts";
import styles from "./Keys.module.css";

export interface KeysProps {
  /** The keys, written as in the registry: `Mod+k`, `g h`. */
  readonly sequence: string;
}

/** The keys of a shortcut, drawn as keys. Screen readers get the sentence form instead. */
export function Keys({ sequence }: KeysProps) {
  const chords = chordsOfSequence(sequence);
  return (
    <span className={styles.keys}>
      <span className={styles.spoken}>{describeKeys(sequence)}</span>
      <span className={styles.drawn} aria-hidden>
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
    </span>
  );
}
