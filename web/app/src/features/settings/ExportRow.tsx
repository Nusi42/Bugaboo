import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  DataCell,
  dataCellVariant,
  DataRow,
  DataRowHead,
  partOf,
  part,
} from "@bugaboo/ui";

import { type DataExport, useDownloadExport } from "../../shared/data-export.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDate } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { formatSize } from "../../shared/file-size.ts";

/** Where a file stands, in words: being made, ready with its size, or failed. */
function stateText(item: DataExport): string {
  if (item.status === "ready") {
    return t("data-exports.file.ready", { size: formatSize(item.bytes ?? 0) });
  }
  return item.status === "waiting" ? t("data-exports.file.waiting") : t("data-exports.file.failed");
}

/** One file: whose data, who made it and why, when, where it stands, and *Download* for the one who asked for it. */
export function ExportRow({ item, mine }: { readonly item: DataExport; readonly mine: boolean }) {
  const whose = personName(item.person);
  const download = useDownloadExport();
  return (
    <DataRow>
      <DataRowHead>
        <span {...partOf(part.itemTitle)}>{whose}</span>
      </DataRowHead>
      <DataCell label={t("data-exports.col.asked-by")}>{personName(item.asked_by)}</DataCell>
      <DataCell label={t("data-exports.col.reason")}>
        {item.reason ?? <span {...partOf(part.muted)}>{t("data-exports.own")}</span>}
      </DataCell>
      <DataCell label={t("data-exports.col.asked-at")} variant={dataCellVariant.date}>
        {formatDate(item.asked_at)}
      </DataCell>
      <DataCell label={t("data-exports.col.file")}>
        <span {...partOf(part.startColumn)}>
          <span>{stateText(item)}</span>
          <span {...partOf(part.muted)}>
            {t("data-exports.deleted-on", { date: formatDate(item.deleted_at) })}
          </span>
          {download.isError ? (
            <Alert tone={alertTone.danger}>{messageOf(download.error)}</Alert>
          ) : null}
        </span>
      </DataCell>
      <DataCell variant={dataCellVariant.action}>
        {mine && item.status === "ready" ? (
          <Button
            aria-label={t("data-exports.download-name", { name: whose })}
            isPending={download.isPending}
            onPress={() => {
              download.mutate(item.id);
            }}
          >
            {t("data-exports.download")}
          </Button>
        ) : null}
      </DataCell>
    </DataRow>
  );
}
