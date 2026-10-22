import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { fileSentence } from "../../shared/file-sentence.ts";
import { WithWording, WordingNotice } from "../../shared/with-wording.tsx";
import { changeSentence, plainSentenceOf, useScrollTo } from "./history.ts";

/** What tells the items of one event apart: the field that changed, the relation or the file. */
function keyOf(item: Schema<"HistoryEntryItem">): string {
  return [item.event, item.field ?? item.relation ?? item.file_event ?? ""].join(" ");
}

/**
 * The changes of one event as one entry: a sentence for each field that changed, and when. A
 * relation that was added or taken away is a sentence too. What the reader may not see is not in
 * the entries at all (SEC-06).
 */
export function ChangeEntry({
  items,
  highlighted = false,
}: {
  readonly items: readonly Schema<"HistoryEntryItem">[];
  /** *Open* of a notification came here. */
  readonly highlighted?: boolean;
}) {
  const shown = useScrollTo<HTMLElement>(highlighted);
  const first = items[0];
  if (first === undefined) {
    return null;
  }
  return (
    <article ref={shown} {...partOf(part.historyEntry, { states: [highlighted && "highlighted"] })}>
      <span {...partOf(part.historyWhen)}>{formatDateTime(first.at)}</span>
      <ul {...partOf(part.list)}>
        {items.map((item) => (
          <li key={keyOf(item)} {...partOf(part.itemText)}>
            {plainSentenceOf(item) ?? (
              <WithWording>
                {(wording) =>
                  item.kind === "file" ? fileSentence(wording, item) : changeSentence(wording, item)
                }
              </WithWording>
            )}
          </li>
        ))}
      </ul>
    </article>
  );
}

/** What the history says when there is nothing in it yet. */
export function NothingYet() {
  return <p {...partOf(part.muted)}>{t("history.nothing")}</p>;
}

/** Says once for the block of changes that their words could not be loaded, with *Try again*. */
export function WordsFailedNotice() {
  return <WordingNotice />;
}
