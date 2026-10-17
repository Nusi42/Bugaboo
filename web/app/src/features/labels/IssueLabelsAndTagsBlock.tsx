import type { IssueBlockProps } from "../issues/index.ts";
import { useIssueLabelsAndTags } from "./labels.ts";
import { LabelsRow, TagsRow } from "./LabelsAndTagsParts.tsx";
import { useLabelChanges } from "./use-label-changes.ts";

/**
 * *Tags* as a name-and-value row of the Details (TAG-01, UX §4.4): the tags the issue has, each
 * with a button that takes it off, and the way to add one where the person may change them. A
 * change is made in place and the row says *Saved* with *Undo* (R7). Tags are a built-in field of
 * the issue page, so the layout decides whether they show and where (`form.shows`), and a person
 * who may not see the field gets nothing of it (SEC-06).
 */
export function IssueTagsBlock({ issue, readableId, form }: IssueBlockProps) {
  const labelsAndTags = useIssueLabelsAndTags(readableId);
  const changes = useLabelChanges(readableId);
  const body = labelsAndTags.data;
  if (body === undefined || !form.shows("tags")) {
    return null;
  }
  return (
    <TagsRow
      body={body}
      changes={changes}
      project={issue.project?.short_name}
      problem={changes.error}
    />
  );
}

/** *Labels* as a name-and-value row of the Details (TAG-02): as `IssueTagsBlock`, for the labels. */
export function IssueLabelsBlock({ readableId, form }: IssueBlockProps) {
  const labelsAndTags = useIssueLabelsAndTags(readableId);
  const changes = useLabelChanges(readableId);
  const body = labelsAndTags.data;
  if (body === undefined || !form.shows("labels")) {
    return null;
  }
  return <LabelsRow body={body} changes={changes} problem={changes.error} />;
}
