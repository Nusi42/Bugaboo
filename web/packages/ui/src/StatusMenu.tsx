import { ActionOverlay, type MenuAction } from "./ActionMenu.tsx";
import { StatusButton, type StatusButtonProps } from "./StatusButton.tsx";

export interface StatusMenuProps extends Pick<StatusButtonProps, "group" | "label" | "id"> {
  /** The name of the list of statuses, for screen readers: *Change the status*. */
  readonly menuLabel: string;
  /** The changes the person can make now, in words. What it offers is for the screen to decide. */
  readonly choices: readonly MenuAction[];
  /** The word of *Cancel* on the sheet of a phone. */
  readonly cancelLabel?: string;
}

/**
 * The big status button of the issue page and the list it opens (UX §4.4): the current status with
 * its group's icon and color, and a list of the changes that can be made right now, under the
 * button on a computer and as a sheet on a phone.
 */
export function StatusMenu({ group, label, id, menuLabel, choices, cancelLabel }: StatusMenuProps) {
  return (
    <ActionOverlay
      label={menuLabel}
      actions={choices}
      {...(cancelLabel === undefined ? {} : { cancelLabel })}
      renderTrigger={(onOpen) => (
        <StatusButton
          group={group}
          label={label}
          {...(id === undefined ? {} : { id })}
          {...(onOpen === undefined ? {} : { onPress: onOpen })}
        />
      )}
    />
  );
}
