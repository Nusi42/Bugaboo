import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { fieldList, type FieldNames } from "../../shared/paged.ts";
import { projectAddress } from "../../shared/paths.ts";
import type { QuickResult, QuickSearchSource } from "../shell/index.ts";
import { lineOf, type ListItem } from "./use-projects.ts";

/** How many projects quick search shows. */
const RESULTS = 8;

/**
 * Projects as a source of quick search (UX §6.2): the projects the person sees whose name or short
 * name holds every word typed. The short name is the hint, so two projects with one name can be
 * told apart.
 */
export const projectsSource: QuickSearchSource = {
  id: "projects",
  heading: t("search.projects"),
  search: async (query): Promise<readonly QuickResult[]> => {
    const { data } = await api.client.GET("/api/v1/projects", {
      params: {
        query: {
          search: query,
          fields: fieldList(["id", "short_name", "name"] satisfies FieldNames<ListItem>),
          limit: RESULTS,
        },
      },
    });
    return (data?.items ?? []).flatMap((item) => {
      const line = lineOf(item);
      return line === undefined
        ? []
        : [
            {
              id: line.id,
              title: line.name,
              path: projectAddress(line.shortName),
              hint: line.shortName,
            },
          ];
    });
  },
};
