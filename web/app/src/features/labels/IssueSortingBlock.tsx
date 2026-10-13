import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import type { IssueBlockProps } from "../issues/index.ts";
import { AddLabel } from "./AddLabel.tsx";
import { AddTag } from "./AddTag.tsx";
import { Chips } from "./Chips.tsx";
import { refreshLabels, useIssueSorting } from "./labels.ts";

/** What takes a tag or a label off the issue. */
function useTakeOff() {
  const tag = api.query.useMutation("delete", "/api/v1/issues/{issue}/tags/{tag}", {
    onSuccess: refreshLabels,
  });
  const label = api.query.useMutation("delete", "/api/v1/issues/{issue}/labels/{label}", {
    onSuccess: refreshLabels,
  });
  const failed = tag.isError ? tag.error : label.isError ? label.error : null;
  return { tag, label, failed };
}

/**
 * *Labels and tags* in the fields panel of the issue page (TAG-01, TAG-02): the tags and labels the
 * issue has, each with a button that takes it off, and the ways to add one where the person may
 * change them. A person who may not see the fields gets nothing here (SEC-06).
 */
export function IssueSortingBlock({ issue, issueKey }: IssueBlockProps) {
  const heading = useId();
  const sorting = useIssueSorting(issueKey);
  const off = useTakeOff();
  if (sorting.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(sorting.error)}</Alert>;
  }
  const body = sorting.data;
  if (body === undefined) {
    return null;
  }
  const project = issue.project?.short_name;
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h3 id={heading}>{t("labels.sorting-title")}</h3>
      <h4>{t("labels.tags-heading")}</h4>
      <Chips
        label={t("labels.tags-heading")}
        chips={body.tags ?? []}
        none={t("labels.none-tags")}
        onTakeOff={
          body.may_change_tags
            ? (tag) => {
                off.tag.mutate({ params: { path: { issue: issueKey, tag: tag.id } } });
              }
            : undefined
        }
      />
      {body.may_change_tags && project !== undefined ? (
        <AddTag issue={issueKey} project={project} />
      ) : null}
      <h4>{t("labels.labels-heading")}</h4>
      <Chips
        label={t("labels.labels-heading")}
        chips={body.labels ?? []}
        none={t("labels.none-labels")}
        onTakeOff={
          body.may_change_labels
            ? (label) => {
                off.label.mutate({ params: { path: { issue: issueKey, label: label.id } } });
              }
            : undefined
        }
      />
      {body.may_change_labels ? <AddLabel issue={issueKey} has={body.labels ?? []} /> : null}
      {off.failed === null ? null : <Alert tone={alertTone.danger}>{messageOf(off.failed)}</Alert>}
    </section>
  );
}
