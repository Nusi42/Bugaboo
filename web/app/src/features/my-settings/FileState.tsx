import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, StatesInPlace, partOf, part } from "@bugaboo/ui";

import { type DataExport, useDownloadExport } from "../../shared/data-export.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDate, formatDateTime } from "../../shared/format.ts";
import { formatSize, widestSize } from "../../shared/file-size.ts";

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

/**
 * A moment with wide words in every *Region* (two-digit day and hour, a long month), for the room of the words of a
 * file before there is one: a sample, never shown.
 */
const WIDE_MOMENT = "2026-12-28T22:58:00Z";

/** The words of a ready file, or, while there is none, of a file with the widest size and wide dates, for its room. */
function readyText(item: DataExport | null): string {
  return t("my-data.ready", {
    date: formatDateTime(item?.ready_at ?? item?.asked_at ?? WIDE_MOMENT),
    size: item === null || item.bytes === null ? widestSize() : formatSize(item.bytes),
    until: formatDate(item?.deleted_at ?? WIDE_MOMENT),
  });
}

/** The two words a ready file may show: its own, and those of the widest file, unseen, which keep the room. */
const READY_WORDS = ["own", "widest"] as const;

/** The words of the file itself, the ones shown. */
const OWN_WORDS = READY_WORDS[0];

/** *Download the file* of a ready file, which the page offers as the notification does. */
function ReadyFile({ item }: { readonly item: DataExport | null }) {
  const download = useDownloadExport();
  return (
    <div {...partOf(part.stack)}>
      {/* The words of the file keep the room of the widest file, so a file that is ready moves nothing (UX §6.4). */}
      <StatesInPlace
        current={OWN_WORDS}
        states={READY_WORDS}
        render={(words) => (
          <p {...partOf(part.text)}>{readyText(words === OWN_WORDS ? item : null)}</p>
        )}
      />
      <div {...partOf(part.row)}>
        <Button
          isPending={download.isPending}
          onPress={() => {
            if (item !== null) {
              download.mutate(item.id);
            }
          }}
        >
          {t("my-data.download-file")}
        </Button>
      </div>
      {download.isError ? <Alert tone={alertTone.danger}>{messageOf(download.error)}</Alert> : null}
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
