import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { refreshIssueFiles } from "../../shared/query-keys.ts";
import { metaOf, type FileItem } from "./file-item.ts";
import { DownloadButton, FileTile } from "./FileLinks.tsx";
import { FileName } from "./FileName.tsx";

/**
 * Brings a deleted file back, as *Undo* asks. The row it was in is gone by then, so this is not a
 * hook of a row: a refusal is told in a note of the same place *Undo* was offered.
 */
async function bringBack(item: FileItem): Promise<void> {
  try {
    await api.client.POST("/api/v1/attachments/{attachment}/restore", {
      params: { path: { attachment: item.id } },
    });
    refreshIssueFiles();
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
}

/** One file: what it is, who added it and when, and what the reader may do with it. */
function FileRow({ item, issue }: { readonly item: FileItem; readonly issue: string }) {
  const remove = api.query.useMutation("delete", "/api/v1/attachments/{attachment}", {
    onSuccess: () => {
      // A deleted file is in the trash: *Undo* is on offer where every *Undo* is (UX §6).
      undoStore.offer({
        message: t("files.deleted", { name: item.name }),
        undo: () => {
          void bringBack(item);
        },
      });
      refreshIssueFiles();
    },
  });
  return (
    <div {...partOf(part.filesRow)}>
      <FileTile item={item} issue={issue} />
      <div {...partOf(part.filesInfo)}>
        <FileName name={item.name} />
        <span {...partOf(part.filesMeta)}>{metaOf(item)}</span>
      </div>
      <div {...partOf(part.filesActions)}>
        <DownloadButton item={item} issue={issue} />
        {item.may_delete ? (
          <Button
            look={buttonLook.quiet}
            isPending={remove.isPending}
            aria-label={t("files.delete-named", { name: item.name })}
            onPress={() => {
              remove.mutate({ params: { path: { attachment: item.id } } });
            }}
          >
            {t("files.delete")}
          </Button>
        ) : null}
      </div>
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </div>
  );
}

/** The files of a list, one row each. */
export function FileRows({
  items,
  issue,
  label,
}: {
  readonly items: readonly FileItem[];
  readonly issue: string;
  readonly label: string;
}) {
  if (items.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.list)} aria-label={label}>
      {items.map((item) => (
        <li key={item.id}>
          <FileRow item={item} issue={issue} />
        </li>
      ))}
    </ul>
  );
}
