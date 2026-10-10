import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useCallback } from "react";

import { api } from "../../shared/api.ts";
import { IssueLongText, type IssueBlockProps, type SaveLongText } from "../issues/index.ts";
import { editPartOf, issueFieldsOf, useFields, type OwnField } from "./fields.ts";

/** One long-text own field of an issue: shown as the server rendered it, edited under its lock. */
function LongTextField({
  field,
  blocks,
}: {
  readonly field: OwnField;
  readonly blocks: IssueBlockProps;
}) {
  const { issue, address, ownFieldsHtml, mask } = blocks;
  const id = issue.id ?? address;
  const fieldId = field.id;
  const save = useCallback<SaveLongText>(
    async (input) => {
      dataOf(
        await api.client.PUT("/api/v1/issues/{issue}/fields/{field}/text", {
          params: { path: { issue: id, field: fieldId } },
          body: { text: input.text, version: input.version, lock_token: input.lockToken },
        }),
      );
    },
    [id, fieldId],
  );
  const text = (issue.own_fields ?? {})[fieldId];
  return (
    <IssueLongText
      issue={issue}
      address={address}
      part={editPartOf(fieldId)}
      text={typeof text === "string" ? text : ""}
      html={ownFieldsHtml.find((each) => each.field === fieldId)?.html}
      canChange={mask.changes(fieldId)}
      words={{ title: field.name, empty: t("fields.long-empty") }}
      save={save}
    />
  );
}

/** The long texts among the own fields of an issue, as sections under the description (CF-01). */
export function OwnFieldSections(blocks: IssueBlockProps) {
  const { issue, mask } = blocks;
  const fields = useFields({ project: issue.project?.short_name });
  return (
    <>
      {issueFieldsOf(fields.data?.fields ?? [], issue)
        .filter(
          (field) => field.kind === "long-text" && (mask.sees(field.id) || mask.changes(field.id)),
        )
        .map((field) => (
          <LongTextField key={field.id} field={field} blocks={blocks} />
        ))}
    </>
  );
}
