import { t } from "@bugaboo/messages";
import { Alert, alertTone, FileDrop, partOf, part } from "@bugaboo/ui";
import { useCallback, useId } from "react";

import { messageOf } from "../../shared/errors.ts";
import { FileRows } from "./FileRows.tsx";
import { useFileLimit, useFileRules, useIssueFiles } from "./files.ts";
import { TransferRows } from "./TransferRows.tsx";
import { transfers, useTransfers } from "./transfers.ts";
import { usePastedFiles } from "./use-pasted-files.ts";

/**
 * *Files* (ATT-01, UX §4.4): a row for each file of the issue, with its preview or the sign of its
 * type, who added it and when, *Download* and *Delete*. Files are dropped on the place at the top,
 * pasted (a screenshot) or chosen with *Upload*; each upload shows its bar where the file will be.
 * Files that belong to a comment are shown under that comment.
 */
export function FilesBlock({ readableId }: { readonly readableId: string }) {
  const heading = useId();
  const rules = useFileRules(readableId);
  const list = useIssueFiles(readableId);
  const sending = useTransfers({ issue: readableId, comment: null });
  const mayAdd = rules.data?.may_add === true;
  const limit = useFileLimit(readableId);
  const add = useCallback(
    (files: readonly File[]) => {
      transfers.add({ issue: readableId, comment: null }, files, limit);
    },
    [readableId, limit],
  );
  usePastedFiles(mayAdd ? add : undefined);
  const items = (list.data?.items ?? []).filter(
    (each) => each.comment === undefined || each.comment === null,
  );
  return (
    <section {...partOf(part.stack)} aria-labelledby={heading}>
      <h2 id={heading} {...partOf(part.issueSectionTitle)}>
        {t("files.title")}
      </h2>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {mayAdd ? (
        <FileDrop label={t("files.drop-place")} chooseLabel={t("files.upload")} onFiles={add}>
          <span>{t("files.drop")}</span>
        </FileDrop>
      ) : null}
      <FileRows items={items} issue={readableId} label={t("files.of-issue")} />
      <TransferRows items={sending} />
      {list.isSuccess && items.length === 0 && sending.length === 0 ? (
        <p {...partOf(part.muted)}>{t("files.none")}</p>
      ) : null}
    </section>
  );
}
