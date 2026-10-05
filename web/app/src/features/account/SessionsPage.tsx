import { t } from "@bugaboo/messages";
import { Alert, Card, SaveStatus } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./Account.module.css";
import { SessionRow } from "./SessionRow.tsx";
import { sessionsOptions } from "./sessions.ts";
import { SignOutEverywhere } from "./SignOutEverywhere.tsx";
import { useEndSession } from "./use-end-session.ts";

/** The sessions page: every place with its browser and system, and the way to sign out everywhere. */
export function SessionsPage() {
  usePageTitle(t("sessions.title"));
  const sessions = useQuery(sessionsOptions());
  const end = useEndSession();
  return (
    <Card heading={t("sessions.title")} level={1}>
      <p className={styles.text}>{t("sessions.intro")}</p>
      {sessions.isError ? <Alert danger>{messageOf(sessions.error)}</Alert> : null}
      <ul className={styles.list} aria-label={t("sessions.list")}>
        {sessions.data?.sessions.map((session) => (
          <SessionRow
            key={session.id}
            session={session}
            onEnd={() => {
              end.change({ id: session.id });
            }}
          />
        ))}
      </ul>
      <SaveStatus
        state={end.saveState}
        labels={{
          saving: t("save.saving"),
          saved: t("save.saved"),
          "not-saved": t("save.not-saved"),
        }}
      />
      {end.problem === null ? null : <Alert danger>{end.problem}</Alert>}
      {end.saveState === "saved" ? <Alert success>{t("sessions.ended")}</Alert> : null}
      {sessions.data?.truncated === true ? <Alert>{t("sessions.cut-off")}</Alert> : null}
      <SignOutEverywhere />
    </Card>
  );
}
