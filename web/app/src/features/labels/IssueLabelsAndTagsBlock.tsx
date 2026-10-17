import { Alert, alertTone } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { useIssueLabelsAndTags } from "./labels.ts";
import { LabelsRow, TagsRow } from "./LabelsAndTagsParts.tsx";
import { useLabelChanges } from "./use-label-changes.ts";

/**
 * *Tags* and *Labels* as name-and-value rows of the Details (TAG-01, TAG-02, UX §4.4): the tags and
 * labels the issue has, each with a button that takes it off, and the ways to add one where the
 * person may change them. A change is made in place and the row says *Saved* with *Undo* (R7).
 * Tags and labels are built-in fields of the issue page, so the layout decides which of them show
 * (`form.shows`, like *Reported by*), and a person who may not see a field gets nothing of it
 * (SEC-06).
 */
export function IssueLabelsAndTagsBlock({ issue, readableId, form }: IssueBlockProps) {
  const labelsAndTags = useIssueLabelsAndTags(readableId);
  const changes = useLabelChanges(readableId);
  const [showTags, showLabels] = [form.shows("tags"), form.shows("labels")];
  const body = labelsAndTags.data;
  const problem = changes.error ?? (labelsAndTags.isError ? labelsAndTags.error : null);
  if (body === undefined || (!showTags && !showLabels)) {
    return problem === null ? null : <Alert tone={alertTone.danger}>{messageOf(problem)}</Alert>;
  }
  return (
    <>
      {showTags ? (
        <TagsRow body={body} changes={changes} project={issue.project?.short_name} />
      ) : null}
      {showLabels ? <LabelsRow body={body} changes={changes} /> : null}
    </>
  );
}
