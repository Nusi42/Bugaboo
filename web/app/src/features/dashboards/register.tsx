import { addSidebarSection } from "../shell/index.ts";
import { DashboardsSection } from "./DashboardsSection.tsx";
import { dashboardsHeading } from "./dashboards.ts";

let registered = false;

/**
 * Puts *Dashboards* into the sidebar and the sheet *More* (CFG-03): the person's dashboards and those shared with them.
 * Called once when the app starts.
 */
export function registerDashboards(): void {
  if (registered) {
    return;
  }
  registered = true;
  addSidebarSection({
    heading: dashboardsHeading,
    render: (view) => <DashboardsSection view={view} />,
  });
}
