import { t } from "@bugaboo/messages";

import { ActionMenu, type ActionMenuTriggerProps, type MenuAction } from "./ActionMenu.tsx";
import { doNothing } from "./do-nothing.ts";
import { IconButton } from "./IconButton.tsx";
import { iconName } from "./icons.ts";

/** The square button of *More* with its icon; it is named by the row's words. */
export function MoreButton({ label, onOpen }: ActionMenuTriggerProps) {
  return <IconButton icon={iconName.more} label={label} onPress={onOpen ?? doNothing} />;
}

export interface ItemActionsProps {
  /** What the actions are about, in words: the Issue ID of a card. The button is named "Actions for WEB-4". */
  readonly name: string;
  /** The entries of the menu, the first one the main one (*Move to …*, *Change due date …*). */
  readonly actions: readonly MenuAction[];
}

/**
 * The compact menu button of a thing that is dragged (a card of the board, an issue on the calendar, a box of a
 * dashboard): a "⋯" as high as a control (36 px with a mouse, 44 px with a finger), and the typed ways of its drag in
 * the menu. One component, so every screen that drags names and sizes it alike (TEST-A11Y, UX §6.4). It stands with
 * the button it opens from, in the components, so the screens that use it share no chunk of their own for it.
 */
export function ItemActions({ name, actions }: ItemActionsProps) {
  return (
    <ActionMenu
      label={t("common.actions-for", { name })}
      actions={actions}
      trigger={MoreButton}
      cancelLabel={t("common.cancel")}
    />
  );
}
