import { refreshRoutes } from "./query-keys.ts";

/** The answers that show work times: one's own, another person's, and the shares of a project. */
const WORK_TIMES_READS = [
  "/api/v1/me/work-times",
  "/api/v1/people/{person}/work-times",
  "/api/v1/projects/{project}/shares",
] as const;

/** Work times or a share changed: every answer that shows them is read again. */
export function refreshWorkTimes(): void {
  refreshRoutes(WORK_TIMES_READS);
}
