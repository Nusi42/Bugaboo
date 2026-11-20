import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, StatesInPlace, partOf, part } from "@bugaboo/ui";

import { downloadExport, type DataExport } from "../../shared/data-export.ts";
import { formatDate, formatDateTime } from "../../shared/format.ts";
import { formatSize } from "../../shared/file-size.ts";

/** Where the file stands: none asked for yet, or the state of the newest. */
type FileShown = "none" | DataExport["status"];

/** Every state the part can be in. */
const FILE_STATES: readonly FileShown[] = ["none", "waiting", "ready", "failed"];

/** What a state of the file shows. */
function shownFor(state: FileShown, item: DataExport | null) {
  if (state === "none") {
    return <p {...partOf(part.muted)}>{t("my-data.none")}</p>;
  }
  if (state === "waiting") {
    return <p {...partOf(part.text)}>{t("my-data.waiting")}</p>;
  }
  return state === "ready" ? (
    <ReadyFile item={item} />
  ) : (
    <Alert tone={alertTone.danger}>{t("my-data.failed")}</Alert>
  );
}

/** The words of a ready file: of the file itself, or, while there is none, of one asked for now, for its room. */
function readyText(item: DataExport | null): string {
  const asked = item?.asked_at ?? new Date().toISOString();
  return t("my-data.ready", {
    date: formatDateTime(item?.ready_at ?? asked),
    size: formatSize(item?.bytes ?? 0),
    until: formatDate(item?.deleted_at ?? asked),
  });
}

/** *Download the file* of a ready file, which the page offers as the notification does. */
function ReadyFile({ item }: { readonly item: DataExport | null }) {
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.text)}>{readyText(item)}</p>
      <div {...partOf(part.row)}>
        <Button
          onPress={() => {
            if (item !== null) {
              void downloadExport(item.id);
            }
          }}
        >
          {t("my-data.download-file")}
        </Button>
      </div>
    </div>
  );
}

/**
 * Where the newest file stands, in words, with *Download the file* while it is there. Every state stands in one place
 * (`StatesInPlace`), so the part keeps its size from the first read on: a file that is being created and then ready
 * moves nothing under it (UX §6.4).
 */
export function FileState({ item }: { readonly item: DataExport | null }) {
  const current: FileShown = item?.status ?? "none";
  return (
    <StatesInPlace
      current={current}
      states={FILE_STATES}
      render={(state) => shownFor(state, item)}
    />
  );
}
