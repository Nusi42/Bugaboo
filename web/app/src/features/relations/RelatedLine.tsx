import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshIssueRelations } from "../../shared/query-keys.ts";
import { IssueRefRow } from "./IssueRefRow.tsx";

/** *Take away* a relation; the page shows the change at once. */
function TakeAway({ relation, label }: { readonly relation: string; readonly label: string }) {
  const remove = api.query.useMutation("delete", "/api/v1/relations/{relation}", {
    onSuccess: refreshIssueRelations,
  });
  return (
    <>
      <Button
        look={buttonLook.quiet}
        aria-label={label}
        isPending={remove.isPending}
        onPress={() => {
          remove.mutate({ params: { path: { relation } } });
        }}
      >
        {t("related-issues.take-away-short")}
      </Button>
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </>
  );
}

/** One relation as a row, in the words of this issue's end. */
export function RelatedLine({
  each,
  mayChange,
}: {
  readonly each: Schema<"RelatedIssueItem">;
  readonly mayChange: boolean;
}) {
  return (
    <IssueRefRow
      issue={each.issue}
      meaning={each.name}
      action={
        mayChange ? (
          <TakeAway
            relation={each.id}
            label={t("related-issues.take-away", { name: each.name, other: each.issue.key })}
          />
        ) : undefined
      }
    />
  );
}
