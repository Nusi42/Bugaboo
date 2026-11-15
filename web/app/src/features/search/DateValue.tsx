import { t } from "@bugaboo/messages";
import { SelectField, TextField } from "@bugaboo/ui";
import type { Operand, TimeWord } from "@bugaboo/wasm-search";

import { dayOf, nothingPicked, type ValueInputProps } from "./filter-draft.ts";
import { dayText, timeWordText } from "./filter-text.ts";
import { nowhere } from "../../shared/search-tree.ts";
import { useTypedDay } from "../../shared/typed-day.ts";

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
  // The day is typed as in every box for a day (TXT-DATES).
  const typed = useTypedDay(first?.type === "date" ? dayText(first.date) : "");
  const dayIn = (text: string) => dayOf(typed.read(text) ?? "");
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
          const day = dayIn(typed.typed);
          onValues(picked === undefined ? (day === null ? [] : [day]) : [timeOperand(picked)]);
        }}
      />
      {word === undefined ? (
        <TextField
          label={t("search.bar.day")}
          description={t("search.bar.day-format")}
          value={typed.typed}
          errorMessage={typed.problem}
          onChange={(text) => {
            typed.setTyped(text);
            const operand = dayIn(text);
            onValues(operand === null ? [] : [operand]);
          }}
        />
      ) : null}
    </>
  );
}
