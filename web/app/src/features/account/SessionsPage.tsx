import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, SaveStatus, Skeleton, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { SessionRow } from "./SessionRow.tsx";
import { SignOutEverywhere } from "./SignOutEverywhere.tsx";
import { useEndSession } from "./use-end-session.ts";
import { useSessionsView } from "./use-sessions-view.ts";
import { saveLabels } from "../../shared/save-labels.ts";

/** The sessions page: every place with its browser and system, and the way to sign out everywhere. */
export function SessionsPage() {
  usePageTitle(t("sessions.title"));
  const { sessions, shapes } = useSessionsView();
  const end = useEndSession();
  return (
    <Card heading={t("sessions.title")} level={1} intro={t("sessions.intro")}>
      {sessions.isError ? <Alert tone={alertTone.danger}>{messageOf(sessions.error)}</Alert> : null}
      <div aria-busy={sessions.isPending}>
        <ul {...partOf(part.list)} aria-label={t("sessions.list")}>
          {sessions.data?.sessions.map((session) => (
            <SessionRow
              key={session.id}
              session={session}
              onEnd={() => {
                end.remove({ id: session.id });
              }}
            />
          ))}
        </ul>
        {shapes ? <Skeleton /> : null}
      </div>
      <SaveStatus state={end.saveState} labels={saveLabels()} />
      {end.problem === null ? null : <Alert tone={alertTone.danger}>{end.problem}</Alert>}
      {end.saveState === "saved" ? (
        <Alert tone={alertTone.success}>{t("sessions.ended")}</Alert>
      ) : null}
      {sessions.data?.truncated === true ? <Alert>{t("sessions.cut-off")}</Alert> : null}
      <SignOutEverywhere />
    </Card>
  );
}
