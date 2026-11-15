import { t } from "@bugaboo/messages";
import { SelectField, TextField } from "@bugaboo/ui";
import type { Operand, TimeWord } from "@bugaboo/wasm-search";
import { useState } from "react";

import { dayOf, nothingPicked, type ValueInputProps } from "./filter-draft.ts";
import { dayText, timeWordText } from "./filter-text.ts";
import { dayText as writtenDay } from "../../shared/format.ts";
import { dayProblem, readDay } from "../../shared/read-day.ts";
import { useRegion } from "../../shared/region.ts";
import { nowhere } from "../../shared/search-tree.ts";

const timeWords: readonly TimeWord[] = [
  "today",
  "yesterday",
  "this_week",
  "last_week",
  "this_month",
];

function timeOperand(word: TimeWord): Operand {
  return { type: "time", word, span: nowhere };
}

/** A day typed, or a time word (*today*, *this week*). */
export function DateValue({ values, onValues }: ValueInputProps) {
  const [first] = values;
  const word = first?.type === "time" ? first.word : undefined;
  const region = useRegion();
  // The day is typed in the person's date format, as in every box for a day (TXT-DATES).
  const [typed, setTyped] = useState(
    first?.type === "date" ? writtenDay(dayText(first.date), region) : "",
  );
  const dayIn = (text: string) => dayOf(readDay(text, region) ?? "");
  return (
    <>
      <SelectField
        label={t("search.bar.when")}
        choices={[
          { value: nothingPicked, label: t("search.bar.a-day") },
          ...timeWords.map((each) => ({
            value: each,
            label: timeWordText(each),
          })),
        ]}
        value={word ?? nothingPicked}
        onChange={(next) => {
          const picked = timeWords.find((each) => each === next);
          const day = dayIn(typed);
          onValues(picked === undefined ? (day === null ? [] : [day]) : [timeOperand(picked)]);
        }}
      />
      {word === undefined ? (
        <TextField
          label={t("search.bar.day")}
          description={t("search.bar.day-format")}
          value={typed}
          errorMessage={dayProblem(typed, region)}
          onChange={(text) => {
            setTyped(text);
            const operand = dayIn(text);
            onValues(operand === null ? [] : [operand]);
          }}
        />
      ) : null}
    </>
  );
}
