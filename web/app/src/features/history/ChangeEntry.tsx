import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  IssueId,
  factSeparator,
  Length,
  partOf,
  part,
  SentenceWithName,
  SentenceWithValue,
  SentenceWithValues,
} from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { fileSentence } from "../../shared/file-sentence.ts";
import { timeSentence } from "../../shared/time-sentence.ts";
import { durationText, minutesText } from "../../shared/own-value-text.ts";
import { WithWording, WordingNotice } from "../../shared/with-wording.tsx";
import { changeSentence, relationSentence, useScrollTo } from "./history.ts";
import { timeFieldValues } from "./time-fields.ts";

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

/** The length of booked time in an item, as its sentence writes it; none without one. */
function timeLengths(item: Schema<"HistoryEntryItem">): readonly string[] {
  return item.minutes === undefined || item.minutes === null ? [] : [minutesText(item.minutes)];
}

/** A side of a change that is a length of time (seconds, as an own field's), as its sentence writes it. */
function lengthOf(side: Schema<"ChangeValueBody"> | null | undefined): readonly string[] {
  return side?.shown === "typed" && side.kind === "duration" && typeof side.value === "number"
    ? [durationText(side.value)]
    : [];
}

/** The lengths of a change (*Estimated time*, a length of an own field), before and after. */
function changeLengths(item: Schema<"HistoryEntryItem">): readonly string[] {
  return [...lengthOf(item.change?.before), ...lengthOf(item.change?.after)];
}

/** No own fields beside a sentence. */
const noFields: readonly string[] = [];

/** The sentence of one item that the shared wording words: a file, time, or a change of a field. */
function WordedItem({
  item,
  fields = noFields,
}: {
  readonly item: Schema<"HistoryEntryItem">;
  /** The own fields of a booked time, said after its sentence. */
  readonly fields?: readonly string[];
}) {
  return (
    <WithWording>
      {(wording) => {
        if (item.kind === "file") {
          return <SentenceWithName sentence={fileSentence(wording, item)} name={item.file_name} />;
        }
        const [sentence, lengths] =
          item.kind === "time"
            ? [timeSentence(wording, item), timeLengths(item)]
            : [changeSentence(wording, item), changeLengths(item)];
        return (
          <>
            <SentenceWithValues sentence={sentence} values={lengths}>
              {(length) => <Length text={length} />}
            </SentenceWithValues>
            {fields.map((field) => ` ${factSeparator} ${field}`)}
          </>
        );
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
  // The own fields of booked time stand in the sentence of the time, not as sentences of their own.
  const ofTime = first.kind === "time";
  const sentences = ofTime ? [first] : items;
  const timeFields = ofTime ? timeFieldValues(items.slice(1)) : [];
  return (
    <article ref={shown} {...partOf(part.historyEntry, { states: [highlighted && "highlighted"] })}>
      <span {...partOf(part.historyWhen)}>{formatDateTime(first.at)}</span>
      <ul {...partOf(part.list)}>
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
