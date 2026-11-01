import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { IssueId, Length, partOf, part, SentenceWithName, SentenceWithValue } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { fileSentence } from "../../shared/file-sentence.ts";
import { timeSentence } from "../../shared/time-sentence.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { WithWording, WordingNotice } from "../../shared/with-wording.tsx";
import { changeSentence, relationSentence, useScrollTo } from "./history.ts";

/** What tells the items of one event apart: the field that changed, the relation, the file or the time. */
function keyOf(item: Schema<"HistoryEntryItem">): string {
  return [
    item.event,
    item.change?.field ?? item.relation_name ?? item.file_event ?? item.time_event ?? "",
  ].join(" ");
}

/** The kinds of item this list has a sentence for; another kind (one a newer server sends) shows nothing. */
const worded: ReadonlySet<Schema<"HistoryEntryKind">> = new Set([
  "change",
  "relation",
  "file",
  "time",
]);

/** The sentence of one item that the shared wording words: a file, time, or a change of a field. */
function WordedItem({ item }: { readonly item: Schema<"HistoryEntryItem"> }) {
  return (
    <WithWording>
      {(wording) => {
        if (item.kind === "file") {
          return <SentenceWithName sentence={fileSentence(wording, item)} name={item.file_name} />;
        }
        if (item.kind === "time") {
          return (
            <SentenceWithValue
              sentence={timeSentence(wording, item)}
              value={
                item.minutes === undefined || item.minutes === null
                  ? undefined
                  : minutesText(item.minutes)
              }
            >
              {(length) => <Length text={length} />}
            </SentenceWithValue>
          );
        }
        return changeSentence(wording, item);
      }}
    </WithWording>
  );
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
        {items
          .filter((item) => worded.has(item.kind))
          .map((item) => (
            <li key={keyOf(item)} {...partOf(part.itemText)}>
              {item.kind === "relation" ? (
                <SentenceWithValue
                  sentence={relationSentence(item)}
                  value={item.issue?.readable_id}
                >
                  {(id) => <IssueId id={id} />}
                </SentenceWithValue>
              ) : (
                <WordedItem item={item} />
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
