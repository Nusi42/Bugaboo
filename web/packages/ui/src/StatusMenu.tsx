import { useState } from "react";

import { ActionOverlay, type MenuAction } from "./ActionMenu.tsx";
import { StatusButton, type StatusButtonProps } from "./StatusButton.tsx";

export interface StatusMenuProps extends Pick<StatusButtonProps, "group" | "label" | "id"> {
  /** The name of the list of statuses, for screen readers: *Change the status*. */
  readonly menuLabel: string;
  /** The changes the person can make now, in words. What it offers is for the screen to decide. */
  readonly choices: readonly MenuAction[];
}

/**
 * The big status button of the issue page and the list it opens (UX §4.4): the current status with
 * its group's icon and color, and a list of the changes that can be made right now.
 */
export function StatusMenu({ group, label, id, menuLabel, choices }: StatusMenuProps) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <StatusButton
        group={group}
        label={label}
        {...(id === undefined ? {} : { id })}
        onPress={() => {
          setOpen(true);
        }}
      />
      <ActionOverlay isOpen={open} onOpenChange={setOpen} label={menuLabel} actions={choices} />
    </>
  );
}
