import { t } from "@bugaboo/messages";
import { IconButton, iconName, IssueId, partOf, part } from "@bugaboo/ui";

import { itemName } from "./item-name.ts";
import type { LookingAtItem } from "./looking-at.ts";

/** What an item looks like: an issue by its ID, a project by its short name, a list as *This list*. */
function ItemLabel({ item }: { readonly item: LookingAtItem }) {
  return item.kind === "issue" ? <IssueId id={item.value} /> : <span>{itemName(item)}</span>;
}

export interface LookingAtProps {
  readonly items: readonly LookingAtItem[];
  readonly onLeaveOut: (key: string) => void;
}

/**
 * *Looking at:* at the top of the panel (UX §4.12): what the assistant may read for the question.
 * Each item can be left out, and the question is then asked without it.
 */
export function LookingAt({ items, onLeaveOut }: LookingAtProps) {
  return (
    <section {...partOf(part.stack)} aria-label={t("assistant.looking-at")}>
      <span {...partOf(part.muted)}>{t("assistant.looking-at")}</span>
      {items.length === 0 ? (
        <span {...partOf(part.muted)}>{t("assistant.looking-nothing")}</span>
      ) : (
        <ul {...partOf(part.assistantItems)}>
          {items.map((item) => (
            <li key={item.key} {...partOf(part.assistantChip)}>
              <ItemLabel item={item} />
              <IconButton
                icon={iconName.close}
                label={t("assistant.leave-out", { name: itemName(item) })}
                onPress={() => {
                  onLeaveOut(item.key);
                }}
              />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
