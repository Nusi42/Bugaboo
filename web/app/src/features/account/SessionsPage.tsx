import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Card,
  DelayedSkeleton,
  SaveStatus,
  partOf,
  part,
  keepsRoom,
  useHoldWhileReading,
} from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { messageOf } from "../../shared/errors.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { SessionRow } from "./SessionRow.tsx";
import { SignOutEverywhere } from "./SignOutEverywhere.tsx";
import { useEndSession } from "./use-end-session.ts";
import { sessionsOptions } from "./sessions.ts";
import { saveLabels } from "../../shared/save-labels.ts";

/** The sessions page: every place with its browser and system, and the way to sign out everywhere. */
export function SessionsPage() {
  usePageTitle(t("sessions.title"));
  const sessions = useQuery(sessionsOptions());
  // The topic waits for the first read of the list with the hold it stands in (UX §6.4).
  useHoldWhileReading(sessions.isLoading);
  const end = useEndSession();
  return (
    <Card
      actions={<SaveStatus state={end.saveState} labels={saveLabels()} variant={keepsRoom} />}
      heading={t("sessions.title")}
      level={1}
      intro={t("sessions.intro")}
    >
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
        {sessions.isPending ? <DelayedSkeleton /> : null}
      </div>
      {end.problem === null ? null : <Alert tone={alertTone.danger}>{end.problem}</Alert>}
      {end.saveState === "saved" ? (
        <Alert tone={alertTone.success}>{t("sessions.ended")}</Alert>
      ) : null}
      {sessions.data?.truncated === true ? <Alert>{t("sessions.cut-off")}</Alert> : null}
      <SignOutEverywhere />
    </Card>
  );
}
