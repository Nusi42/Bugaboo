import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshIssueRelations } from "../../shared/query-keys.ts";
import { IssueRefLine } from "./IssueRefLine.tsx";

/** *Take away* a relation; the page shows the change at once. */
function TakeAway({ relation, label }: { readonly relation: string; readonly label: string }) {
  const remove = api.query.useMutation("delete", "/api/v1/relations/{relation}", {
    onSuccess: refreshIssueRelations,
  });
  return (
    <>
      <Button
        look={buttonLook.quiet}
        isPending={remove.isPending}
        onPress={() => {
          remove.mutate({ params: { path: { relation } } });
        }}
      >
        {label}
      </Button>
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </>
  );
}

/** One relation, in the words of this issue's end. */
export function RelatedLine({
  each,
  mayChange,
}: {
  readonly each: Schema<"RelatedIssueItem">;
  readonly mayChange: boolean;
}) {
  return (
    <li {...partOf(part.item)}>
      <span {...partOf(part.muted)}>{each.name}</span>
      <IssueRefLine issue={each.issue}>
        {mayChange ? (
          <TakeAway
            relation={each.id}
            label={t("related-issues.take-away", { name: each.name, other: each.issue.key })}
          />
        ) : null}
      </IssueRefLine>
    </li>
  );
}
