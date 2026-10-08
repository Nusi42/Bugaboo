import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";

type Session = Schema<"SessionBody">;

/** What the person calls a session: the browser and the system, as far as they are known. */
function deviceOf(session: Session): string {
  const { browser, system } = session;
  if (browser === null || browser === undefined || system === null || system === undefined) {
    return browser ?? system ?? t("sessions.unknown-device");
  }
  return t("sessions.device", { browser, system });
}

/** One place where the person is signed in, with a button to sign out of it. */
export interface SessionRowProps {
  readonly session: Session;
  /** Sign out of this place: it leaves the list at once and the server is asked in the background. */
  readonly onEnd: () => void;
}

export function SessionRow({ session, onEnd }: SessionRowProps) {
  const device = deviceOf(session);
  return (
    <li {...partOf(part.accountItem)}>
      <span {...partOf(part.accountItemText)}>
        <span {...partOf(part.accountItemTitle)}>{device}</span>
        {session.current ? <span {...partOf(part.muted)}>{t("sessions.this")}</span> : null}
        <span {...partOf(part.muted)}>
          {t("sessions.started", { when: formatDateTime(session.started_at) })}
        </span>
        <span {...partOf(part.muted)}>
          {t("sessions.last-seen", { when: formatDateTime(session.last_seen_at) })}
        </span>
        {session.network === null || session.network === undefined ? null : (
          <span {...partOf(part.muted)}>{t("sessions.network", { network: session.network })}</span>
        )}
      </span>
      {session.current ? null : (
        <Button aria-label={t("sessions.end-name", { device })} onPress={onEnd}>
          {t("sessions.end")}
        </Button>
      )}
    </li>
  );
}
