import { t } from "@bugaboo/messages";
import { Button, ListGrid, partOf, part, ShowMoreOf } from "@bugaboo/ui";
import { useId, useState } from "react";

import { IssueReadProblem } from "../../shared/IssueReadProblem.tsx";
import { itemsOf } from "../../shared/paged.ts";
import { AddRelation } from "./AddRelation.tsx";
import { RelatedLine } from "./RelatedLine.tsx";
import { useHeldWhileChanging } from "./relation-change.ts";
import { useIssueRelations, useRelatedIssues, type BlockProps } from "./relations.ts";

/** *Add relation*, and the three steps behind it once it is pressed. */
function Adding({ readableId }: BlockProps) {
  const [adding, setAdding] = useState(false);
  if (adding) {
    return (
      <AddRelation
        readableId={readableId}
        onClose={() => {
          setAdding(false);
        }}
      />
    );
  }
  return (
    <div {...partOf(part.row)}>
      <Button
        onPress={() => {
          setAdding(true);
        }}
      >
        {t("related-issues.add")}
      </Button>
    </div>
  );
}

/**
 * *Related issues* (ISS-04): each relation in the words of this issue's end (*blocks*, *is blocked
 * by*), only where the viewer may see the issue at the other end, with *Add relation*.
 */
export function RelatedIssuesBlock({ readableId }: BlockProps) {
  const heading = useId();
  const header = useIssueRelations(readableId);
  const list = useRelatedIssues(readableId);
  const related = itemsOf(useHeldWhileChanging(list.data?.pages));
  const mayChange = useHeldWhileChanging(header.data)?.may_change === true;
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h2 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("related-issues.title")}
      </h2>
      {list.isError ? <IssueReadProblem error={list.error} /> : null}
      {/* One box for the words of no relations and for the list that takes their place: the browser keeps it where it
          stands when it grows, so the first relation pushes the form down and nothing above it moves (UX §6.4). */}
      <div>
        {related.length === 0 ? (
          <p {...partOf(part.muted)}>{t("related-issues.none")}</p>
        ) : (
          <ListGrid label={t("related-issues.title")}>
            {related.map((each) => (
              <RelatedLine key={each.id} each={each} mayChange={mayChange} />
            ))}
          </ListGrid>
        )}
      </div>
      <ShowMoreOf label={t("related-issues.more")} list={list} />
      {mayChange ? <Adding readableId={readableId} /> : null}
    </section>
  );
}
