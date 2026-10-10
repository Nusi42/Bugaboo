import { Menu, MenuItem, MenuTrigger, Popover } from "react-aria-components";

import { Button } from "./Button.tsx";
import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

export interface MenuAction {
  /** Names the action in lists and tests; unique in the menu. */
  readonly id: string;
  /** What it does, in words. */
  readonly label: string;
  readonly onAction: () => void;
}

export interface ActionMenuProps {
  /** The button that opens the menu: *More*. */
  readonly label: string;
  readonly actions: readonly MenuAction[];
}

/**
 * A button that opens a short list of actions (UX §4.4: *More*). It works with the keyboard and a
 * screen reader as React Aria makes it, closes with Esc and gives the focus back to the button.
 */
export function ActionMenu({ label, actions }: ActionMenuProps) {
  return (
    <MenuTrigger>
      <Button>{label}</Button>
      <Popover {...partOf(part.actionMenuPopover)}>
        <Menu {...partOf(part.actionMenu)} aria-label={label}>
          {actions.map((action) => (
            <MenuItem
              key={action.id}
              id={action.id}
              {...partOf(part.actionMenuItem)}
              onAction={action.onAction}
            >
              {action.label}
            </MenuItem>
          ))}
        </Menu>
      </Popover>
    </MenuTrigger>
  );
}
