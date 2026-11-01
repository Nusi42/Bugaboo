import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { totalsText } from "./time-words.ts";

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
  const words = totals.data === undefined ? "" : totalsText(totals.data);
  return words === "" ? null : (
    <p {...partOf(part.muted)}>{t("issues.project-time", { totals: words })}</p>
  );
}
