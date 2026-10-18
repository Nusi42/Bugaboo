import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { metaOf, type FileItem } from "./file-item.ts";
import { DownloadButton, FileTile } from "./FileLinks.tsx";
import { refreshIssueFiles } from "../../shared/query-keys.ts";

/** One file: what it is, who added it and when, and what the reader may do with it. */
function FileRow({
  item,
  issue,
  onDeleted,
}: {
  readonly item: FileItem;
  readonly issue: string;
  readonly onDeleted: (item: FileItem) => void;
}) {
  const remove = api.query.useMutation("delete", "/api/v1/attachments/{attachment}", {
    onSuccess: () => {
      onDeleted(item);
      refreshIssueFiles();
    },
  });
  return (
    <div {...partOf(part.filesRow)}>
      <FileTile item={item} issue={issue} />
      <div {...partOf(part.filesInfo)}>
        <span {...partOf(part.filesName)}>{item.name}</span>
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

/** A file that was just deleted: it is in the trash, and *Undo* brings it back. */
function DeletedFile({
  item,
  onRestored,
}: {
  readonly item: FileItem;
  readonly onRestored: (item: FileItem) => void;
}) {
  const restore = api.query.useMutation("post", "/api/v1/attachments/{attachment}/restore", {
    onSuccess: () => {
      onRestored(item);
      refreshIssueFiles();
    },
  });
  return (
    <Alert tone={alertTone.info}>
      <span>{t("files.deleted", { name: item.name })}</span>
      <Button
        look={buttonLook.quiet}
        isPending={restore.isPending}
        onPress={() => {
          restore.mutate({ params: { path: { attachment: item.id } } });
        }}
      >
        {t("common.undo")}
      </Button>
      {restore.isError ? <span>{messageOf(restore.error)}</span> : null}
    </Alert>
  );
}

/** The files of a list, one row each, with the ones just deleted still there to take back. */
export function FileRows({
  items,
  issue,
  label,
}: {
  readonly items: readonly FileItem[];
  readonly issue: string;
  readonly label: string;
}) {
  const [deleted, setDeleted] = useState<readonly FileItem[]>([]);
  const gone = deleted.filter((each) => !items.some((item) => item.id === each.id));
  if (items.length === 0 && gone.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.list)} aria-label={label}>
      {items.map((item) => (
        <li key={item.id}>
          <FileRow
            item={item}
            issue={issue}
            onDeleted={(removed) => {
              setDeleted((all) => [...all, removed]);
            }}
          />
        </li>
      ))}
      {gone.map((removed) => (
        <li key={removed.id}>
          <DeletedFile
            item={removed}
            onRestored={(back) => {
              setDeleted((all) => all.filter((each) => each.id !== back.id));
            }}
          />
        </li>
      ))}
    </ul>
  );
}
