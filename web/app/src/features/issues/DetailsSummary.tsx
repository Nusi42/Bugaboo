import { t } from "@bugaboo/messages";

import { listText } from "../../shared/list-text.ts";
import { personName } from "../../shared/person.ts";
import { known } from "../../shared/present.ts";
import type { IssueView } from "./slots.ts";

/** The values a closed *Details* still says (UX §3.3): who it is assigned to and the priority, such as *Ana · High*. */
export function detailsSummary(issue: IssueView): string {
  const people = listText((issue.assigned_to ?? []).map((person) => personName(person)));
  const priority = known(issue.priority);
  return [people, priority === undefined ? "" : t(`priority.${priority}`)]
    .filter((word) => word !== "")
    .join(t("issues.summary-separator"));
}
