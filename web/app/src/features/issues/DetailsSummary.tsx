import { t } from "@bugaboo/messages";
import { Priority } from "@bugaboo/ui";

import { listText } from "../../shared/list-text.ts";
import { personName } from "../../shared/person.ts";
import { known } from "../../shared/present.ts";
import type { IssueView } from "./slots.ts";

/**
 * The values a closed *Details* still says (UX §3.3): who the issue is assigned to, in words
 * (*Not assigned* when nobody is), and its priority with its icon, such as *Ana · ↑ High*.
 */
export function DetailsSummary({ issue }: { readonly issue: IssueView }) {
  const people = (issue.assigned_to ?? []).map((person) => personName(person));
  const priority = known(issue.priority);
  return (
    <>
      <span>{people.length === 0 ? t("issues.summary-unassigned") : listText(people)}</span>
      {priority === undefined ? null : (
        <>
          <span>{t("issues.summary-separator")}</span>
          <Priority priority={priority} label={t(`priority.${priority}`)} />
        </>
      )}
    </>
  );
}
