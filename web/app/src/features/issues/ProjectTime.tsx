import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { TotalFacts } from "./TimeFacts.tsx";
import { hasTotals } from "./time-words.ts";

/**
 * The totals of the time of a project under its count line (TIME-03): its issues and those of its
 * sub-projects that the person sees, of the fields they may read (*Time in this project: 12h
 * estimated · 9h 30m spent*). Nothing while there is nothing to add up.
 */
export function ProjectTime({ project }: { readonly project: string }) {
  const totals = useQuery({
    queryKey: queryKeys.issues.time("project", project),
    queryFn: async () =>
      (
        await api.client.GET("/api/v1/projects/{project}/time", {
          params: { path: { project } },
        })
      ).data,
  });
  return totals.data === undefined || !hasTotals(totals.data) ? null : (
    <p {...partOf(part.muted)}>
      {/* One run of text: the line of the list may lay out its children side by side. */}
      <span>
        {t("issues.project-time")} <TotalFacts totals={totals.data} />
      </span>
    </p>
  );
}
