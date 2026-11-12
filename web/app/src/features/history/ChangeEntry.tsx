import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { IssueId, partOf, part, SentenceWithValue } from "@bugaboo/ui";

import { whenOf } from "./when.ts";
import { WordingNotice } from "../../shared/with-wording.tsx";
import { relationSentence, useScrollTo } from "./history.ts";
import { RunMark } from "./RunMark.tsx";
import { DatesMovedItem, WordedItem } from "./WordedItem.tsx";
import { timeFieldValues } from "./time-fields.ts";
import { dateItems, datesMoveOf, type DatesMove } from "../../shared/dates-sentence.ts";

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

/** The line of both dates moving in one event. */
function DatesMovedLine({
  item,
  move,
}: {
  readonly item: Schema<"HistoryEntryItem">;
  readonly move: DatesMove;
}) {
  return (
    <li {...partOf(part.itemText)}>
      <DatesMovedItem item={item} move={move} />
      <RunMark item={item} />
    </li>
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
  // The own fields of booked time stand in the sentence of the time, not as sentences of their own.
  const ofTime = first.kind === "time";
  // Both dates moving in one event are one sentence; each of them alone is said on its own.
  const move = ofTime ? undefined : datesMoveOf(items);
  const moved = new Set(move === undefined ? [] : dateItems(items));
  const sentences = ofTime ? [first] : items.filter((item) => !moved.has(item));
  const timeFields = ofTime ? timeFieldValues(items.slice(1)) : [];
  return (
    <article ref={shown} {...partOf(part.historyEntry, { states: [highlighted && "highlighted"] })}>
      <span {...partOf(part.historyWhen)}>{whenOf(first)}</span>
      <ul {...partOf(part.list)}>
        {move === undefined ? null : <DatesMovedLine item={first} move={move} />}
        {sentences
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
                <WordedItem item={item} fields={timeFields} />
              )}
              <RunMark item={item} />
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
