import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Checkbox, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { QueryGate } from "../shell/index.ts";

/** The issue types, for the ones a field can be limited to. */
function useIssueTypeChoices() {
  return useQuery({
    ...api.query.queryOptions("get", "/api/v1/issue-types"),
    select: (page: Schema<"Page_IssueTypeItem">) => page,
  });
}

/** The issue types a field can be limited to, as boxes to tick. */
export function IssueTypeBoxes({
  types,
  setTypes,
}: {
  readonly types: readonly string[];
  readonly setTypes: (types: readonly string[]) => void;
}) {
  const issueTypes = useIssueTypeChoices();
  return (
    <QueryGate error={issueTypes.error} data={issueTypes.data}>
      {(page) => (
        <fieldset {...partOf(part.choiceGroup)}>
          <legend {...partOf(part.choiceGroupLegend)}>{t("fields.scope-types")}</legend>
          <p {...partOf(part.muted)}>{t("fields.scope-types-help")}</p>
          {page.items.map((each) => (
            <Checkbox
              key={each.id ?? ""}
              label={each.name ?? ""}
              isSelected={types.includes(each.id ?? "")}
              onChange={(on) => {
                setTypes(on ? [...types, each.id ?? ""] : types.filter((id) => id !== each.id));
              }}
            />
          ))}
        </fieldset>
      )}
    </QueryGate>
  );
}
