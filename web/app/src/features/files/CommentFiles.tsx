import { t } from "@bugaboo/messages";
import { FilePicker, partOf, part } from "@bugaboo/ui";

import { FileRows } from "./FileRows.tsx";
import { useIssueFiles } from "./files.ts";
import { TransferRows } from "./TransferRows.tsx";
import { useAddFiles, useTransfers } from "./transfers.ts";

/**
 * The files of a comment, in the same rows as the *Files* block, under the text of the comment.
 * The person who wrote the comment (`mayAdd`) can add more with *Add files*.
 */
export function CommentFiles({
  issue,
  comment,
  mayAdd,
}: {
  readonly issue: string;
  readonly comment: string;
  readonly mayAdd: boolean;
}) {
  const list = useIssueFiles(issue);
  const sending = useTransfers({ issue, comment });
  const items = (list.data?.items ?? []).filter((each) => each.comment === comment);
  const add = useAddFiles(issue, comment);
  if (items.length === 0 && sending.length === 0 && !mayAdd) {
    return null;
  }
  return (
    <div {...partOf(part.stack)}>
      <FileRows items={items} issue={issue} label={t("files.of-comment")} />
      <TransferRows items={sending} />
      {mayAdd ? (
        <div {...partOf(part.row)}>
          <FilePicker chooseLabel={t("files.add-to-comment")} onFiles={add} />
        </div>
      ) : null}
    </div>
  );
}
