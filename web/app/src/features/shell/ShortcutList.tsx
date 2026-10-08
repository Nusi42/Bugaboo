import { t } from "@bugaboo/messages";
import type { ReactNode } from "react";
import { partOf, part } from "@bugaboo/ui";

import {
  definitionOf,
  keysOf,
  shortcutDefinitions,
  type ShortcutId,
  type ShortcutOverrides,
} from "../../shared/shortcuts.ts";
import { Keys } from "./Keys.tsx";

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
    <ul {...partOf(part.shortcutListList)} aria-label={t("shortcut.title")}>
      {shortcutDefinitions
        .filter((definition) => show?.(definition.id) ?? true)
        .map((definition) => (
          <li key={definition.id} {...partOf(part.shortcutListRow)}>
            <span {...partOf(part.shortcutListWhat)}>
              <span {...partOf(part.shortcutListName)}>{t(definition.message)}</span>
              <span {...partOf(part.shortcutListWhere)}>
                {t(
                  definitionOf(definition.id).where === "lists"
                    ? "shortcut.in-lists"
                    : "shortcut.everywhere",
                )}
              </span>
            </span>
            <span {...partOf(part.shortcutListKeys)}>
              {keysOf(definition.id, overrides).map((keys, index) => (
                <span key={keys} {...partOf(part.shortcutListAlternative)}>
                  {index === 0 ? null : (
                    <span {...partOf(part.shortcutListOr)}>{t("shortcut.or")}</span>
                  )}
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
