import { Icon, iconName, NameValueRow, partOf, part, type ChangeStatus } from "@bugaboo/ui";
import type { ReactNode } from "react";

export interface DetailRowProps {
  /** The name of the field, in words. */
  readonly name: string;
  /** The value, or what changes it. */
  readonly children: ReactNode;
  /** Why the person cannot change it, when they cannot: shown with a lock. */
  readonly reason?: string | undefined;
  /** What the last change of this field came to: *Saved* and *Undo*, where it happened. */
  readonly status?: ChangeStatus | undefined;
}

/**
 * One field of the Details (UX §4.4): its name on the left, its value or the way to change it on
 * the right, and, when it cannot be changed by this person, a lock and the reason.
 */
export function DetailRow({ name, children, reason, status }: DetailRowProps) {
  return (
    <NameValueRow name={name} {...(status === undefined ? {} : { status })}>
      {children}
      {reason === undefined ? null : (
        <span {...partOf(part.nameValueReason)}>
          <Icon name={iconName.lock} />
          <span>{reason}</span>
        </span>
      )}
    </NameValueRow>
  );
}
