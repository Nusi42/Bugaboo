import { t } from "@bugaboo/messages";
import { part, partOf } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { usePageTitle } from "../../shared/use-page.ts";
import { QueryGate } from "../shell/index.ts";
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
  usePageTitle(read.data?.name ?? t("dashboards.heading"));
  return (
    <QueryGate error={read.error} data={read.data} notFound={notFound}>
      {(dashboard: Dashboard) => (
        <div {...partOf(part.dashboard)}>
          <DashboardHead dashboard={dashboard} />
          <DashboardBoxes key={dashboard.id} dashboard={dashboard} />
        </div>
      )}
    </QueryGate>
  );
}
