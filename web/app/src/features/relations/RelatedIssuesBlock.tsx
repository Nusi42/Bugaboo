import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, ListGrid, partOf, part, ShowMoreOf } from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { AddRelation } from "./AddRelation.tsx";
import { RelatedLine } from "./RelatedLine.tsx";
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
  const related = itemsOf(list.data?.pages);
  const mayChange = header.data?.may_change === true;
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h2 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("related-issues.title")}
      </h2>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {related.length === 0 ? (
        <p {...partOf(part.muted)}>{t("related-issues.none")}</p>
      ) : (
        <ListGrid label={t("related-issues.title")}>
          {related.map((each) => (
            <RelatedLine key={each.id} each={each} mayChange={mayChange} />
          ))}
        </ListGrid>
      )}
      {<ShowMoreOf label={t("related-issues.more")} list={list} />}
      {mayChange ? <Adding readableId={readableId} /> : null}
    </section>
  );
}
