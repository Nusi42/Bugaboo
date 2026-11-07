import type { Schema } from "@bugaboo/api-client";
import { factJoinerHeld, Length, SentenceWithName, SentenceWithValues } from "@bugaboo/ui";

import { fileSentence } from "../../shared/file-sentence.ts";
import { durationText, minutesText } from "../../shared/own-value-text.ts";
import { timeSentence } from "../../shared/time-sentence.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { changeSentence } from "./history.ts";

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
export function WordedItem({
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
          <span>
            <SentenceWithValues sentence={sentence} values={lengths}>
              {(length) => <Length text={length} />}
            </SentenceWithValues>
            {fields.map((field) => `${factJoinerHeld}${field}`)}
          </span>
        );
      }}
    </WithWording>
  );
}
