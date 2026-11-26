import { ActionMenu, type ActionMenuTriggerProps, type MenuAction } from "./ActionMenu.tsx";
import { doNothing } from "./do-nothing.ts";
import { IconButton } from "./IconButton.tsx";
import { iconName } from "./icons.ts";

/** The square button of *More* with its icon; it is named by the row's words. */
export function MoreButton({ label, onOpen }: ActionMenuTriggerProps) {
  return <IconButton icon={iconName.more} label={label} onPress={onOpen ?? doNothing} />;
}

export interface ItemActionsProps {
  /** The name of the button, from what the actions are about: "Actions for WEB-4". */
  readonly label: string;
  /** The words of the button that closes the menu as a sheet on a phone (*Cancel*). */
  readonly cancelLabel: string;
  /** The entries of the menu, the first one the main one (*Move to …*, *Change due date …*). */
  readonly actions: readonly MenuAction[];
}

/**
 * The compact menu button of a thing that is dragged (a card of the board, an issue on the calendar, a box of a
 * dashboard): a "⋯" as high as a control (36 px with a mouse, 44 px with a finger), and the typed ways of its drag in
 * the menu. One component, so every screen that drags names and sizes it alike (TEST-A11Y, UX §6.4). It stands with
 * the button it opens from, in the components, so the screens that use it share no chunk of their own for it. Its words
 * come from the screen, which has them.
 */
export function ItemActions({ label, cancelLabel, actions }: ItemActionsProps) {
  return (
    <ActionMenu label={label} actions={actions} trigger={MoreButton} cancelLabel={cancelLabel} />
  );
}
