import { t } from "@bugaboo/messages";

import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { known } from "../../shared/present.ts";
import { DetailRow } from "./DetailRow.tsx";
import type { IssueBlockProps } from "./slots.ts";

/**
 * The fields of the Details that nobody changes (UX §4.4): who reported the issue and when it was
 * made and last changed. A field the person may not see is left out.
 */
export function FixedRows({ blocks }: { readonly blocks: IssueBlockProps }) {
  const { issue, form } = blocks;
  const [reportedBy, createdAt, updatedAt] = [
    known(issue.reported_by),
    known(issue.created_at),
    known(issue.updated_at),
  ];
  return (
    <>
      {reportedBy === undefined || !form.shows("reported-by") ? null : (
        <DetailRow name={t("issue-field.reported-by")}>{personName(reportedBy)}</DetailRow>
      )}
      {createdAt === undefined ? null : (
        <DetailRow name={t("issues.created-on")}>{formatDateTime(createdAt)}</DetailRow>
      )}
      {updatedAt === undefined ? null : (
        <DetailRow name={t("issues.updated")}>{formatDateTime(updatedAt)}</DetailRow>
      )}
    </>
  );
}
