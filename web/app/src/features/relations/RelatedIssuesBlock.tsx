import { t } from "@bugaboo/messages";
import { Button, Link, ListGrid, partOf, part, ShowMoreOf, useHoldWhileReading } from "@bugaboo/ui";
import { useId, useState } from "react";

import { IssueReadProblem } from "../../shared/IssueReadProblem.tsx";
import { issueMapAddress } from "../../shared/paths.ts";
import { itemsOf } from "../../shared/paged.ts";
import { AddRelation } from "./AddRelation.tsx";
import { RelatedLine } from "./RelatedLine.tsx";
import { useHeldWhileChanging } from "./relation-change.ts";
import { useIssueRelations, useRelatedIssues, type BlockProps } from "./relations.ts";

/**
 * *Add relation* where the person may add one, and the three steps behind it once it is pressed;
 * and *Show relation map* (UX §4.4), which everybody who sees the issue may open.
 */
function Actions({ readableId, mayChange }: BlockProps & { readonly mayChange: boolean }) {
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
      {mayChange ? (
        <Button
          onPress={() => {
            setAdding(true);
          }}
        >
          {t("related-issues.add")}
        </Button>
      ) : null}
      <Link href={issueMapAddress(readableId)} button>
        {t("relation-map.show")}
      </Link>
    </div>
  );
}

/**
 * *Related issues* (ISS-04): each relation in the words of this issue's end (*blocks*, *is blocked
 * by*), only where the viewer may see the issue at the other end, with *Add relation* and *Show
 * relation map*.
 */
export function RelatedIssuesBlock({ readableId }: BlockProps) {
  const heading = useId();
  const header = useIssueRelations(readableId);
  const list = useRelatedIssues(readableId);
  useHoldWhileReading(header.isLoading || list.isLoading);
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
      <Actions readableId={readableId} mayChange={mayChange} />
    </section>
  );
}
