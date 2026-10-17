import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { messageOf } from "../../shared/errors.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { useIssueLabelsAndTags } from "./labels.ts";
import { LabelsPart, TagsPart } from "./LabelsAndTagsParts.tsx";

/**
 * *Labels and tags* in the Details of the issue page (TAG-01, TAG-02): the tags and labels the
 * issue has, each with a button that takes it off, and the ways to add one where the person may
 * change them. Tags and labels are built-in fields of the issue page, so the layout decides which
 * of them show (`form.shows`, like *Reported by*), and a person who may not see a field gets
 * nothing of it (SEC-06).
 */
export function IssueLabelsAndTagsBlock({ issue, readableId, form }: IssueBlockProps) {
  const heading = useId();
  const labelsAndTags = useIssueLabelsAndTags(readableId);
  const [showTags, showLabels] = [form.shows("tags"), form.shows("labels")];
  if (labelsAndTags.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(labelsAndTags.error)}</Alert>;
  }
  const body = labelsAndTags.data;
  if (body === undefined || (!showTags && !showLabels)) {
    return null;
  }
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h3 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("labels.labels-and-tags-title")}
      </h3>
      {showTags ? (
        <TagsPart body={body} readableId={readableId} project={issue.project?.short_name} />
      ) : null}
      {showLabels ? (
        <LabelsPart body={body} readableId={readableId} project={issue.project?.short_name} />
      ) : null}
    </section>
  );
}
