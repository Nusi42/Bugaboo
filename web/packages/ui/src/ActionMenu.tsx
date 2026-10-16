import { useState, type ReactNode } from "react";

import { Button, type ButtonProps } from "./Button.tsx";
import { useLayout } from "./layout.ts";
import { Overlay } from "./Overlay.tsx";
import { partOf } from "./parts.ts";
import { buttonLook, overlayEdge } from "./props.ts";
import { part } from "./tokens.ts";

export interface MenuAction {
  /** Names the action in lists and tests; unique in the menu. */
  readonly id: string;
  /** What it does, in words. */
  readonly label: string;
  readonly onAction: () => void;
}

/** What a button that opens the menu is given. */
export interface ActionMenuTriggerProps {
  /** The name of the menu: what the button is called. */
  readonly label: string;
  readonly onOpen: () => void;
}

export interface ActionMenuProps {
  /** The button that opens the menu: *More*. */
  readonly label: string;
  readonly actions: readonly MenuAction[];
  readonly look?: ButtonProps["look"];
  /** Another button to open the menu with, such as the control of a title bar. */
  readonly trigger?: (props: ActionMenuTriggerProps) => ReactNode;
}

/**
 * A button that opens a short list of actions (UX §4.4: *More*). The list opens in the one overlay
 * of the app (a sheet from the bottom on a phone), takes the focus, closes with Esc and gives the
 * focus back to the button; pressing an action does it and closes the list.
 */
export function ActionMenu({
  label,
  actions,
  look = buttonLook.plain,
  trigger: Trigger,
}: ActionMenuProps) {
  const [open, setOpen] = useState(false);
  const phone = useLayout() === "phone";
  const openMenu = (): void => {
    setOpen(true);
  };
  return (
    <>
      {Trigger === undefined ? (
        <Button look={look} onPress={openMenu}>
          {label}
        </Button>
      ) : (
        <Trigger label={label} onOpen={openMenu} />
      )}
      <Overlay
        isOpen={open}
        onOpenChange={setOpen}
        label={label}
        from={phone ? overlayEdge.bottom : overlayEdge.center}
      >
        <ul {...partOf(part.actionMenu)}>
          {actions.map((action) => (
            <li key={action.id}>
              <Button
                look={buttonLook.quiet}
                wide
                onPress={() => {
                  setOpen(false);
                  action.onAction();
                }}
              >
                {action.label}
              </Button>
            </li>
          ))}
        </ul>
      </Overlay>
    </>
  );
}
