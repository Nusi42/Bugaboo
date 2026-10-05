import { t } from "@bugaboo/messages";
import type { ReactNode } from "react";

import {
  definitionOf,
  keysOf,
  shortcutDefinitions,
  type ShortcutId,
  type ShortcutOverrides,
} from "../../shared/shortcuts.ts";
import { Keys } from "./Keys.tsx";
import styles from "./ShortcutList.module.css";

export interface ShortcutListProps {
  /** The keys the person chose instead of the defaults. */
  readonly overrides: ShortcutOverrides;
  /** Something after each row, such as the button that changes the keys. */
  readonly action?: (id: ShortcutId) => ReactNode;
  /** Only the shortcuts this says yes to: a search in the list. */
  readonly show?: (id: ShortcutId) => boolean;
}

/** Every shortcut of the registry with its keys and where it works (UX §6.2). */
export function ShortcutList({ overrides, action, show }: ShortcutListProps) {
  return (
    <ul className={styles.list} aria-label={t("shortcut.list")}>
      {shortcutDefinitions
        .filter((definition) => show?.(definition.id) ?? true)
        .map((definition) => (
          <li key={definition.id} className={styles.row}>
            <span className={styles.what}>
              <span className={styles.name}>{t(definition.message)}</span>
              <span className={styles.where}>
                {t(
                  definitionOf(definition.id).where === "lists"
                    ? "shortcut.in-lists"
                    : "shortcut.everywhere",
                )}
              </span>
            </span>
            <span className={styles.keys}>
              {keysOf(definition.id, overrides).map((keys, index) => (
                <span key={keys} className={styles.alternative}>
                  {index === 0 ? null : <span className={styles.or}>{t("shortcut.or")}</span>}
                  <Keys sequence={keys} />
                </span>
              ))}
            </span>
            {action?.(definition.id)}
          </li>
        ))}
    </ul>
  );
}
