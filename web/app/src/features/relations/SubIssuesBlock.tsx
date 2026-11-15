import { t } from "@bugaboo/messages";
import { ListGrid, partOf, part, ShowMoreOf, useHoldWhileReading } from "@bugaboo/ui";
import { useId } from "react";

import { AddSubIssue, PartOf, TakeOut } from "./ParentChange.tsx";
import { IssueReadProblem } from "../../shared/IssueReadProblem.tsx";
import { itemsOf } from "../../shared/paged.ts";
import type { ShownIssue } from "./shown-issue.ts";
import { IssueRefRow } from "./IssueRefRow.tsx";
import { useHeldWhileChanging } from "./relation-change.ts";
import { useIssueRelations, useSubIssues, type BlockProps } from "./relations.ts";

/** How far the sub-issues are, in a bar and in words. */
function Progress({ done, total }: { readonly done: number; readonly total: number }) {
  return (
    <div {...partOf(part.row)}>
      <progress
        {...partOf(part.progress)}
        aria-label={t("sub-issues.progress-label")}
        value={done}
        max={total}
      />
      <span>{t("sub-issues.progress", { done, total })}</span>
    </div>
  );
}

/** The parts the viewer may see, one line each. */
function Parts({
  parts,
  mayChange,
}: {
  readonly parts: readonly ShownIssue[];
  readonly mayChange: boolean;
}) {
  if (parts.length === 0) {
    return <p {...partOf(part.muted)}>{t("sub-issues.none")}</p>;
  }
  return (
    <ListGrid label={t("sub-issues.title")}>
      {parts.map((sub) => (
        <IssueRefRow
          key={sub.readable_id}
          issue={sub}
          action={
            mayChange ? (
              <TakeOut
                child={sub.readable_id}
                label={t("sub-issues.take-out", { key: sub.readable_id })}
              />
            ) : undefined
          }
        />
      ))}
    </ListGrid>
  );
}

/**
 * *Sub-issues* (ISS-03): the issue this one sits under, how far its parts are (counting only the
 * ones the viewer may see), the parts, and *Add sub-issue*.
 */
export function SubIssuesBlock({ readableId }: BlockProps) {
  const heading = useId();
  const header = useIssueRelations(readableId);
  const list = useSubIssues(readableId);
  useHoldWhileReading(header.isLoading || list.isLoading);
  const shown = useHeldWhileChanging(header.data);
  const pages = useHeldWhileChanging(list.data?.pages);
  const above = shown?.part_of;
  const progress = shown?.progress;
  const mayChange = shown?.may_change === true;
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h2 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("sub-issues.title")}
      </h2>
      {header.isError ? <IssueReadProblem error={header.error} /> : null}
      {above === undefined || above === null ? null : (
        <PartOf above={above} readableId={readableId} mayChange={mayChange} />
      )}
      {progress === undefined || progress === null ? null : (
        <Progress done={progress.done} total={progress.total} />
      )}
      <Parts parts={itemsOf(pages)} mayChange={mayChange} />
      <ShowMoreOf label={t("sub-issues.more")} list={list} />
      {mayChange ? <AddSubIssue readableId={readableId} /> : null}
    </section>
  );
}
