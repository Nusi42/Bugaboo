import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { OldWorkDays } from "./OldWorkDays.tsx";
import { OrganizationTopic } from "./OrganizationTopic.tsx";
import { WhoSeesWorkingHours } from "./WhoSeesWorkingHours.tsx";

/** The settings that belong to *Working hours* (UX §5.1): the rules, the keep period, *Totals only*, and *Billable*. */
const TOPICS = "working-hours.,time.";

/**
 * *Organization settings › Work › Working hours* (TIME-09, TIME-10, TIME-12, ADR-0018, ADR-0035): the
 * settings from the registry in one generic form, then the work days past the keep period with
 * *Delete them*, and who holds *See working hours of others*. Every change is in the security log.
 */
export function WorkingHoursPage() {
  usePageTitle(t("nav.working-hours"));
  return (
    <>
      <Card
        heading={t("nav.working-hours")}
        level={1}
        intro={t("setting.page.working-hours-intro")}
      >
        <OrganizationTopic topics={TOPICS} searchable={false} />
      </Card>
      <OldWorkDays />
      <WhoSeesWorkingHours />
    </>
  );
}
