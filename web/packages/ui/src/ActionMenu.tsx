import { useContext, useState, type ReactNode } from "react";
import { Dialog, DialogTrigger, OverlayTriggerStateContext, Popover } from "react-aria-components";

import { Button, type ButtonProps } from "./Button.tsx";
import { useLayout } from "./layout.ts";
import { Overlay } from "./Overlay.tsx";
import { partOf } from "./parts.ts";
import { buttonLook, overlayEdge } from "./props.ts";
import type { StatusGroup } from "./status.ts";
import { StatusIcon } from "./StatusIcon.tsx";
import { part } from "./tokens.ts";

export interface MenuAction {
  /** Names the action in lists and tests; unique in the menu. */
  readonly id: string;
  /** What it does, in words. */
  readonly label: string;
  /** The group of the status the action leads to: its icon stands before the words (R5). */
  readonly group?: StatusGroup;
  readonly onAction: () => void;
}

/** What a button that opens the menu is given. */
export interface ActionMenuTriggerProps {
  /** The name of the menu: what the button is called. */
  readonly label: string;
  /** Opens the menu. A computer opens the list under the button by itself, so it gives none there. */
  readonly onOpen: (() => void) | undefined;
}

export interface ActionOverlayProps {
  /** The name of the list, for screen readers; on a phone it is the title of the sheet too. */
  readonly label: string;
  readonly actions: readonly MenuAction[];
  /** The word of the button that closes the sheet of a phone without choosing: *Cancel*. */
  readonly cancelLabel?: string;
  /** The button that opens the list; it gets what to do when it is pressed (nothing on a computer). */
  readonly renderTrigger: (onOpen?: () => void) => ReactNode;
}

/** Where the list opens on a computer: under its button, from the start edge (a word of React Aria). */
const below = "bottom start";

/** The actions as a list of buttons that start at the start edge; each closes the list when it is pressed. */
function ActionList({
  actions,
  close,
}: {
  readonly actions: readonly MenuAction[];
  readonly close: () => void;
}) {
  return (
    <ul {...partOf(part.actionMenu)}>
      {actions.map((action) => (
        <li key={action.id}>
          <Button
            look={buttonLook.item}
            wide
            onPress={() => {
              close();
              action.onAction();
            }}
          >
            {action.group === undefined ? null : <StatusIcon group={action.group} />}
            {action.label}
          </Button>
        </li>
      ))}
    </ul>
  );
}

/** The list inside the popover of a computer: it closes the popover, which the trigger above it keeps the state of. */
function ListInPopover({ actions }: { readonly actions: readonly MenuAction[] }) {
  const state = useContext(OverlayTriggerStateContext);
  return (
    <ActionList
      actions={actions}
      close={() => {
        state?.close();
      }}
    />
  );
}

/**
 * The button of a menu and the list of its actions: on a computer the list opens under the button; on
 * a phone it is a sheet from the bottom with its title and *Cancel* (UX §3.3). The list takes the
 * focus, closes with Esc and gives the focus back; pressing an action does it and closes the list.
 * `ActionMenu` and the status menu both use it.
 */
export function ActionOverlay({ label, actions, cancelLabel, renderTrigger }: ActionOverlayProps) {
  const phone = useLayout() === "phone";
  const [open, setOpen] = useState(false);
  if (!phone) {
    return (
      <DialogTrigger>
        {renderTrigger()}
        <Popover placement={below} {...partOf(part.popover)}>
          <Dialog aria-label={label} {...partOf(part.overlayDialog)}>
            <ListInPopover actions={actions} />
          </Dialog>
        </Popover>
      </DialogTrigger>
    );
  }
  const close = (): void => {
    setOpen(false);
  };
  return (
    <>
      {renderTrigger(() => {
        setOpen(true);
      })}
      <Overlay isOpen={open} onOpenChange={setOpen} label={label} from={overlayEdge.bottom}>
        <p {...partOf(part.actionMenuTitle)}>{label}</p>
        <ActionList actions={actions} close={close} />
        {cancelLabel === undefined ? null : (
          <Button wide onPress={close}>
            {cancelLabel}
          </Button>
        )}
      </Overlay>
    </>
  );
}

export interface ActionMenuProps {
  /** The button that opens the menu: *More*. */
  readonly label: string;
  readonly actions: readonly MenuAction[];
  readonly look?: ButtonProps["look"];
  /** Another button to open the menu with, such as the control of a title bar. */
  readonly trigger?: (props: ActionMenuTriggerProps) => ReactNode;
  /** The word of *Cancel* on the sheet of a phone. */
  readonly cancelLabel?: string;
}

/** A button that opens a short list of actions (UX §4.4: *More*), and gives the focus back to the button. */
export function ActionMenu({
  label,
  actions,
  look = buttonLook.plain,
  trigger: Trigger,
  cancelLabel,
}: ActionMenuProps) {
  return (
    <ActionOverlay
      label={label}
      actions={actions}
      {...(cancelLabel === undefined ? {} : { cancelLabel })}
      renderTrigger={(onOpen) =>
        Trigger === undefined ? (
          <Button look={look} {...(onOpen === undefined ? {} : { onPress: onOpen })}>
            {label}
          </Button>
        ) : (
          <Trigger label={label} onOpen={onOpen} />
        )
      }
    />
  );
}
