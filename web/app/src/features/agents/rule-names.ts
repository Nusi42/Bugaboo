import { t } from "@bugaboo/messages";
import { useQueries } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { useProjectChoices, type Rule } from "./agents.ts";
import { projectChoicesOf, projectKey } from "./project-choices.ts";

/** The names a standing rule is told by: its saved search and its project. */
export interface RuleNames {
  readonly search: (id: string) => string;
  readonly project: (id: string) => string;
}

/** The projects the person can give agents work in, by ID: the one list of projects a person can name (shared with access keys). */
export function useProjectNames(): ReadonlyMap<string, string> {
  const choices = projectChoicesOf(useProjectChoices().data?.items ?? [], projectKey.id);
  return new Map(choices.map((choice) => [choice.value, choice.label]));
}

/** The names of the saved searches and projects of these rules, read once each. */
export function useRuleNames(rules: readonly Rule[]): RuleNames {
  const ids = [...new Set(rules.map((rule) => rule.search))];
  const lists = useQueries({
    queries: ids.map((search) =>
      api.query.queryOptions("get", "/api/v1/saved-searches/{search}", {
        params: { path: { search } },
      }),
    ),
  });
  const searches = new Map(ids.map((id, at) => [id, lists[at]?.data?.name] as const));
  const projects = useProjectNames();
  return {
    search: (id) => searches.get(id) ?? t("agents.rule-search-gone"),
    project: (id) => projects.get(id) ?? t("agents.rule-project-gone"),
  };
}
