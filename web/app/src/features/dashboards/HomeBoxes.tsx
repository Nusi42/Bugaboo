import { Alert, alertTone, HoldWhileLoading, part, partOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";

import { DashboardBoxes } from "./DashboardBoxes.tsx";
import { useHomeDashboard } from "./dashboards.ts";

/**
 * The boxes of *Home* (ADR-0050, decision 6; UX §4.1): the person's own dashboard below the fixed blocks, made at the
 * first visit from the blocks *Home* showed before (*My timer*, *My open issues*, *Due soon*); *Customize* is its
 * *Change*. *Home* is never shared and has no *Save a copy*.
 */
export function HomeBoxes() {
  const home = useHomeDashboard();
  if (home.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(home.error)}</Alert>;
  }
  if (home.data === undefined) {
    return <HoldWhileLoading />;
  }
  return (
    <div {...partOf(part.dashboard)}>
      <DashboardBoxes key={home.data.id} dashboard={home.data} />
    </div>
  );
}
