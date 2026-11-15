import { t } from "@bugaboo/messages";
import { Alert, alertTone, DelayedSkeleton, useHoldWhileReading } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { useMySettingsReading } from "../../shared/my-settings.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { EntryDetails } from "./EntryDetails.tsx";
import { useIssues } from "./use-issues.ts";
import { rangeOf } from "./use-week.ts";
import { Week, type WeekProps } from "./Week.tsx";

export type SheetProps = Omit<WeekProps, "issues" | "wording">;

/** One's own week: the work-day rows, the time on issues and the footer. */
export function OwnSheet(props: Omit<SheetProps, "person" | "changing">) {
  const [details, setDetails] = useState<string | undefined>();
  const { issues, ready, failure } = useIssues(props.first, setDetails);
  // A read that could not be made ends the wait: the week shows and says why (UX §6.4).
  useHoldWhileReading(!ready && failure === null);
  const row = issues.rows.find((each) => each.issue === details);
  // The times are written in the person's region: the week waits for it, with grey shapes.
  if (useMySettingsReading()) {
    return <DelayedSkeleton />;
  }
  return (
    <>
      <WithWording>
        {(wording) => (
          <>
            <Week {...props} wording={wording} issues={issues} />
            {row === undefined ? null : (
              <EntryDetails
                row={row}
                days={rangeOf(props.first).days}
                wording={wording}
                onClose={() => {
                  setDetails(undefined);
                }}
              />
            )}
          </>
        )}
      </WithWording>
      {failure === null ? null : <Alert tone={alertTone.danger}>{messageOf(failure)}</Alert>}
      {ready || failure !== null ? null : (
        <Alert tone={alertTone.info}>{t("common.loading")}</Alert>
      )}
    </>
  );
}

/** Another person's week: the work-day rows only (*Working hours of others*). */
export function OtherSheet(props: SheetProps) {
  return <WithWording>{(wording) => <Week {...props} wording={wording} />}</WithWording>;
}
