import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { addressInProject, issueAddress, issueTargetAddress, paths } from "../../shared/paths.ts";
import { failedRenewalProjectOf, targetOf, type NotificationRow } from "./notifications.ts";

export interface NotificationActionsProps {
  readonly row: NotificationRow;
  /** What the notification says, for the names of its buttons. */
  readonly what: string;
  /** *Mark as read* for this notification. */
  readonly onRead: (id: string) => void;
  /** Reading is on its way. */
  readonly isReading: boolean;
  /** The row's *More* and the actions a swipe brought in, on a phone. */
  readonly more: ReactNode;
}

/** *Open the project settings* for a renewal of issue IDs that gave up, where the person may know the project. */
function ProjectSettingsButton({ row }: { readonly row: NotificationRow }) {
  const navigate = useNavigate();
  const shortName = failedRenewalProjectOf(row);
  if (shortName === undefined) {
    return null;
  }
  return (
    <Button
      onPress={() => {
        void navigate({ href: addressInProject(paths.projectSettings, shortName) });
      }}
    >
      {t("notifications.open-project-settings")}
    </Button>
  );
}

/**
 * The buttons of a notification: *Open*, the one for a renewal that gave up, and *Mark as read*; on a phone also the
 * row's *More*, which holds what a swipe brings in.
 */
export function NotificationActions({
  row,
  what,
  onRead,
  isReading,
  more,
}: NotificationActionsProps) {
  const navigate = useNavigate();
  const [key, target] = [row.issue?.readable_id, targetOf(row)];
  return (
    <div {...partOf(part.notificationActions)}>
      {key === undefined ? null : (
        <Button
          aria-label={t("notifications.open-name", { what })}
          onPress={() => {
            // An entry that Open names scrolls itself into view; the router must not put the page at
            // its top after it, which it would if the entry were drawn before the router is done.
            void navigate({
              href: target === undefined ? issueAddress(key) : issueTargetAddress(key, target),
              resetScroll: target === undefined,
            });
          }}
        >
          {t("notifications.open")}
        </Button>
      )}
      <ProjectSettingsButton row={row} />
      {row.read ? null : (
        <Button
          look={buttonLook.quiet}
          isPending={isReading}
          aria-label={t("notifications.mark-read-name", { what })}
          onPress={() => {
            onRead(row.id);
          }}
        >
          {t("notifications.mark-read")}
        </Button>
      )}
      {more}
    </div>
  );
}
