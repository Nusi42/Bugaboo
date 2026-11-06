import { t } from "@bugaboo/messages";
import { SelectField, TextField } from "@bugaboo/ui";
import type { Operand, TimeWord } from "@bugaboo/wasm-search";
import { useState } from "react";

import {
  dayOf,
  decimalOf,
  nothingPicked,
  numberTyped,
  textOperand,
  type ValueInputProps,
} from "./filter-draft.ts";
import { dayText, timeWordText } from "./filter-text.ts";
import { IssueChoice } from "./IssueChoice.tsx";
import { ChoiceBoxes, ChoiceSelect, NamedValue } from "./PickedInputs.tsx";
import { CALENDAR, unbroken } from "../../shared/format.ts";
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

/** A text or a number, typed: the value is only kept while it is one the field takes. */
function TypedValue({ field, values, onValues }: ValueInputProps) {
  const isNumber = field.kind === "number";
  const [first] = values;
  const [typed, setTyped] = useState(
    isNumber ? numberTyped(first) : first?.type === "text" ? first.text : "",
  );
  const operand = (text: string): readonly Operand[] => {
    if (text.trim() === "") {
      return [];
    }
    const decimal = decimalOf(text);
    if (isNumber) {
      return decimal === null ? [] : [{ type: "number", number: decimal, span: nowhere }];
    }
    return [textOperand(text.trim())];
  };
  return (
    <TextField
      label={t("search.bar.value")}
      value={typed}
      errorMessage={
        isNumber && typed.trim() !== "" && decimalOf(typed) === null
          ? t("search.bar.not-a-number")
          : undefined
      }
      onChange={(text) => {
        setTyped(text);
        onValues(operand(text));
      }}
    />
  );
}

/** A length of time typed (`2 hours`, `1 hour 30 minutes`): the search language reads it. */
function DurationValue({ language, values, onValues }: ValueInputProps) {
  const [first] = values;
  // The examples are written by the language, so the words are the ones it reads.
  // An example never breaks across two lines (*1 hour / 30 minutes*): its blanks do not break.
  const example = (minutes: number): string =>
    unbroken(language.operandSource({ type: "duration", minutes, span: nowhere }));
  const examples = {
    hours: example(120),
    minutes: example(45),
    both: example(90),
  };
  // The language writes and reads the length of time, in its own words.
  const [typed, setTyped] = useState(
    first?.type === "duration" ? language.operandSource(first) : "",
  );
  const read = (text: string): Operand | null => {
    const minutes = language.durationMinutesOf(text);
    return minutes === undefined ? null : { type: "duration", minutes, span: nowhere };
  };
  const operand = read(typed);
  return (
    <TextField
      label={t("search.bar.value")}
      description={t("search.bar.duration-help", examples)}
      value={typed}
      errorMessage={
        typed.trim() !== "" && operand === null
          ? t("search.bar.not-a-duration", examples)
          : undefined
      }
      onChange={(text) => {
        setTyped(text);
        const found = read(text);
        onValues(found === null ? [] : [found]);
      }}
    />
  );
}

/** A day typed, or a time word (*today*, *this week*). */
function DateValue({ values, onValues }: ValueInputProps) {
  const [first] = values;
  const word = first?.type === "time" ? first.word : undefined;
  const [typed, setTyped] = useState(first?.type === "date" ? dayText(first.date) : "");
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
          const day = dayOf(typed);
          onValues(picked === undefined ? (day === null ? [] : [day]) : [timeOperand(picked)]);
        }}
      />
      {word === undefined ? (
        <TextField
          label={t("search.bar.day")}
          description={t("search.bar.day-format")}
          value={typed}
          date={CALENDAR}
          onChange={(day) => {
            setTyped(day);
            const operand = dayOf(day);
            onValues(operand === null ? [] : [operand]);
          }}
        />
      ) : null}
    </>
  );
}

/** The inputs for the values of a filter on `field`: what the kind of the field takes. */
export function ValueInputs(props: ValueInputProps & { readonly many: boolean }) {
  const { field, many } = props;
  if (field.choices.length > 0) {
    return many ? <ChoiceBoxes {...props} /> : <ChoiceSelect {...props} />;
  }
  if (field.kind === "date") {
    return <DateValue {...props} />;
  }
  if (field.kind === "duration") {
    return <DurationValue {...props} />;
  }
  if (field.kind === "issue") {
    return (
      <IssueChoice language={props.language} values={props.values} onValues={props.onValues} />
    );
  }
  if (field.kind === "person" || field.kind === "project") {
    return <NamedValue {...props} />;
  }
  return <TypedValue {...props} />;
}
