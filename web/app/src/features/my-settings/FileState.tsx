import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { downloadExport, type DataExport } from "../../shared/data-export.ts";
import { formatDate, formatDateTime } from "../../shared/format.ts";
import { formatSize } from "../../shared/file-size.ts";

/** Where the newest file stands, in words, with *Download the file* while it is there. */
export function FileState({ item }: { readonly item: DataExport | null }) {
  if (item === null) {
    return <p {...partOf(part.muted)}>{t("my-data.none")}</p>;
  }
  if (item.status === "waiting") {
    return <p {...partOf(part.text)}>{t("my-data.waiting")}</p>;
  }
  if (item.status === "failed") {
    return <Alert tone={alertTone.danger}>{t("my-data.failed")}</Alert>;
  }
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.text)}>
        {t("my-data.ready", {
          date: formatDateTime(item.ready_at ?? item.asked_at),
          size: formatSize(item.bytes ?? 0),
          until: formatDate(item.deleted_at),
        })}
      </p>
      <div {...partOf(part.row)}>
        <Button
          onPress={() => {
            void downloadExport(item.id);
          }}
        >
          {t("my-data.download-file")}
        </Button>
      </div>
    </div>
  );
}
