import { filled, paths, routeParam } from "./paths.ts";

/** The address of a dashboard, `/d/<ID>`: its own address, which is shared (ADR-0050, decision 1). */
export function dashboardAddress(id: string): string {
  return filled(paths.dashboard, routeParam.dashboard, id);
}
