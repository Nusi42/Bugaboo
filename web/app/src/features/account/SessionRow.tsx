import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, Button } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
import styles from "./Account.module.css";
import { refreshSessions } from "./sessions.ts";

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
  /** The place was signed out of. */
  readonly onEnded: () => void;
}

export function SessionRow({ session, onEnded }: SessionRowProps) {
  const end = api.query.useMutation("delete", "/api/v1/auth/sessions/{session_id}", {
    onSuccess: () => {
      refreshSessions();
      onEnded();
    },
  });
  const device = deviceOf(session);
  return (
    <li className={styles.item}>
      <span className={styles.itemText}>
        <span className={styles.itemTitle}>{device}</span>
        {session.current ? <span className={styles.muted}>{t("sessions.this")}</span> : null}
        <span className={styles.muted}>
          {t("sessions.started", { when: formatDateTime(session.started_at) })}
        </span>
        <span className={styles.muted}>
          {t("sessions.last-seen", { when: formatDateTime(session.last_seen_at) })}
        </span>
        {session.network === null || session.network === undefined ? null : (
          <span className={styles.muted}>
            {t("sessions.network", { network: session.network })}
          </span>
        )}
        {end.isError ? <Alert danger>{messageOf(end.error)}</Alert> : null}
      </span>
      {session.current ? null : (
        <Button
          aria-label={t("sessions.end-name", { device })}
          isPending={end.isPending}
          onPress={() => {
            end.mutate({ params: { path: { session_id: session.id } } });
          }}
        >
          {t("sessions.end")}
        </Button>
      )}
    </li>
  );
}
