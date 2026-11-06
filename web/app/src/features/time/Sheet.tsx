import { t } from "@bugaboo/messages";
import { Alert, alertTone } from "@bugaboo/ui";

import { WithWording } from "../../shared/with-wording.tsx";
import { useIssues } from "./use-issues.ts";
import { Week, type WeekProps } from "./Week.tsx";

export type SheetProps = Omit<WeekProps, "issues" | "wording">;

/** One's own week: the work-day rows, the time on issues and the footer. */
export function OwnSheet(props: Omit<SheetProps, "person" | "changing">) {
  const { issues, ready } = useIssues(props.first);
  return (
    <>
      <WithWording>
        {(wording) => <Week {...props} wording={wording} issues={issues} />}
      </WithWording>
      {ready ? null : <Alert tone={alertTone.info}>{t("common.loading")}</Alert>}
    </>
  );
}

/** Another person's week: the work-day rows only (*Working hours of others*). */
export function OtherSheet(props: SheetProps) {
  return <WithWording>{(wording) => <Week {...props} wording={wording} />}</WithWording>;
}
