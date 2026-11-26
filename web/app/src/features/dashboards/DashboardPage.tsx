import { t } from "@bugaboo/messages";
import { part, partOf, ReadingHold } from "@bugaboo/ui";
import { useMemo, type ReactNode } from "react";

import { dashboardAddress } from "../../shared/places.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate, usePageTrail } from "../shell/index.ts";
import { DashboardBoxes } from "./DashboardBoxes.tsx";
import { DashboardHead } from "./DashboardHead.tsx";
import { useDashboard, type Dashboard } from "./dashboards.ts";

export interface DashboardPageProps {
  readonly id: string;
  /** What a dashboard the person may not open shows: the page of an address that does not exist. */
  readonly notFound: ReactNode;
}

/**
 * The page of a dashboard, `/d/<ID>` (CFG-03, ADR-0050): its head and its boxes, each read as the person who looks at
 * it. One the person may not open answers like an address that does not exist.
 */
export function DashboardPage({ id, notFound }: DashboardPageProps) {
  const read = useDashboard(id);
  const name = read.data?.name;
  usePageTitle(name ?? t("dashboards.heading"));
  // The top bar says where the page is: *Dashboards* and its name. *Dashboards* has no page of its own (the list is in
  // the sidebar), so it is a name only, not a link.
  const trail = useMemo(
    () =>
      name === undefined
        ? undefined
        : [{ label: t("dashboards.heading") }, { label: name, href: dashboardAddress(id) }],
    [name, id],
  );
  usePageTrail(trail);
  return (
    <QueryGate error={read.error} data={read.data} notFound={notFound}>
      {(dashboard: Dashboard) => (
        // The head (its pin) and the boxes, which grow with what they hold, are shown together once their first reads
        // are in, so nothing pushes another after the page is seen (UX §6.4).
        <ReadingHold>
          <div {...partOf(part.dashboard)}>
            <DashboardHead dashboard={dashboard} />
            <DashboardBoxes key={dashboard.id} dashboard={dashboard} />
          </div>
        </ReadingHold>
      )}
    </QueryGate>
  );
}
