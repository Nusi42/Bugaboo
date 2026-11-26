import { t } from "@bugaboo/messages";
import { ActionMenu, MoreButton, type MenuAction } from "@bugaboo/ui";

export interface ItemActionsProps {
  /** What the actions are about, in words: the Issue ID of a card. The button is named "Actions for WEB-4". */
  readonly name: string;
  /** The entries of the menu, the first one the main one (*Move to …*, *Change due date …*). */
  readonly actions: readonly MenuAction[];
}

/**
 * The compact menu button of a thing that is dragged (a card of the board, an issue on the calendar): a
 * "⋯" as high as a control (36 px with a mouse, 44 px with a finger), and the typed ways of its drag
 * in the menu. One component, so every screen that drags names and sizes it alike (TEST-A11Y, UX §6.4).
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
