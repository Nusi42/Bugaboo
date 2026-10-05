import { t } from "@bugaboo/messages";
import { Alert, Card } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./Account.module.css";
import { SessionRow } from "./SessionRow.tsx";
import { sessionsOptions } from "./sessions.ts";
import { SignOutEverywhere } from "./SignOutEverywhere.tsx";

/** The sessions page: every place with its browser and system, and the way to sign out everywhere. */
export function SessionsPage() {
  usePageTitle(t("sessions.title"));
  const sessions = useQuery(sessionsOptions());
  const [ended, setEnded] = useState(false);
  return (
    <Card heading={t("sessions.title")} level={1}>
      <p className={styles.text}>{t("sessions.intro")}</p>
      {sessions.isError ? <Alert danger>{messageOf(sessions.error)}</Alert> : null}
      <ul className={styles.list} aria-label={t("sessions.list")}>
        {sessions.data?.sessions.map((session) => (
          <SessionRow
            key={session.id}
            session={session}
            onEnded={() => {
              setEnded(true);
            }}
          />
        ))}
      </ul>
      {ended ? <Alert success>{t("sessions.ended")}</Alert> : null}
      {sessions.data?.truncated === true ? <Alert>{t("sessions.cut-off")}</Alert> : null}
      <SignOutEverywhere />
    </Card>
  );
}
