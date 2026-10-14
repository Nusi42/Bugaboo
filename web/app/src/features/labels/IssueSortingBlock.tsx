import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { messageOf } from "../../shared/errors.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { useIssueSorting } from "./labels.ts";
import { LabelsPart, TagsPart } from "./SortingParts.tsx";

/**
 * *Labels and tags* in the fields panel of the issue page (TAG-01, TAG-02): the tags and labels the
 * issue has, each with a button that takes it off, and the ways to add one where the person may
 * change them. Tags and labels are built-in fields of the issue page, so the layout decides which
 * of them show (`form.shows`, like *Reported by*), and a person who may not see a field gets
 * nothing of it (SEC-06).
 */
export function IssueSortingBlock({ issue, issueKey, form }: IssueBlockProps) {
  const heading = useId();
  const sorting = useIssueSorting(issueKey);
  const [showTags, showLabels] = [form.shows("tags"), form.shows("labels")];
  if (sorting.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(sorting.error)}</Alert>;
  }
  const body = sorting.data;
  if (body === undefined || (!showTags && !showLabels)) {
    return null;
  }
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h3 id={heading}>{t("labels.sorting-title")}</h3>
      {showTags ? (
        <TagsPart body={body} issueKey={issueKey} project={issue.project?.short_name} />
      ) : null}
      {showLabels ? <LabelsPart body={body} issueKey={issueKey} /> : null}
    </section>
  );
}
