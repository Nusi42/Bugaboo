import { t } from "@bugaboo/messages";
import { ItemActions, type MenuAction } from "@bugaboo/ui";

import { itemActionsWords } from "../../shared/item-actions-words.ts";

/** What the menu of a box can do; a phone moves a box up and down, a computer by *Move …* and *Resize …*. */
export interface BoxActions {
  readonly onMove?: (() => void) | undefined;
  readonly onResize?: (() => void) | undefined;
  readonly onUp?: (() => void) | undefined;
  readonly onDown?: (() => void) | undefined;
  readonly onChange: () => void;
  readonly onRemove: () => void;
}

/** The actions of the menu, in the order of UX §4.10: the ways to place it, *Change box*, *Remove*. */
function actionsOf(actions: BoxActions): MenuAction[] {
  const offered: (MenuAction | undefined)[] = [
    actions.onMove === undefined
      ? undefined
      : { id: "move", label: t("dashboards.move"), onAction: actions.onMove },
    actions.onResize === undefined
      ? undefined
      : { id: "resize", label: t("dashboards.resize"), onAction: actions.onResize },
    actions.onUp === undefined
      ? undefined
      : { id: "up", label: t("dashboards.move-up"), onAction: actions.onUp },
    actions.onDown === undefined
      ? undefined
      : { id: "down", label: t("dashboards.move-down"), onAction: actions.onDown },
    { id: "change", label: t("dashboards.change-box"), onAction: actions.onChange },
    { id: "remove", label: t("dashboards.remove"), onAction: actions.onRemove },
  ];
  return offered.filter((action) => action !== undefined);
}

/**
 * The menu of a box in *Change* (UX §4.10): *Move …* and *Resize …* (the typed ways of the two drags) on a computer,
 * *Move up* and *Move down* on a phone, then *Change box* and *Remove*; the shared menu of a thing that is dragged.
 */
export function BoxMenu({
  title,
  actions,
}: {
  readonly title: string;
  readonly actions: BoxActions;
}) {
  return <ItemActions {...itemActionsWords(title)} actions={actionsOf(actions)} />;
}
