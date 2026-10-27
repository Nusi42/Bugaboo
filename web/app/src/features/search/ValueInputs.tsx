import { t } from "@bugaboo/messages";
import { Checkbox, SelectField, TextField, part, partOf } from "@bugaboo/ui";
import type { Field, Operand, SearchLanguage, TimeWord } from "@bugaboo/wasm-search";
import { useState } from "react";

import { dayOf, decimalOf, numberTyped, textOperand } from "./filter-draft.ts";
import { dayText, operandText, timeWordText } from "./filter-text.ts";
import { IssueChoice } from "./IssueChoice.tsx";
import { nowhere } from "../../shared/search-tree.ts";

export interface ValueInputProps {
  readonly field: Field;
  readonly language: SearchLanguage;
  readonly values: readonly Operand[];
  readonly onValues: (values: readonly Operand[]) => void;
}

const timeWords: readonly TimeWord[] = [
  "today",
  "yesterday",
  "this_week",
  "last_week",
  "this_month",
];

/** The word of a choice that is not one: the first line of a list with nothing picked yet. */
const nothingPicked = "";

function timeOperand(word: TimeWord): Operand {
  return { type: "time", word, span: nowhere };
}

/** Several choices of a fixed list, ticked (`status in (Open, Done)`). */
function ChoiceBoxes({ field, values, onValues }: ValueInputProps) {
  const chosen = new Set(values.map((value) => operandText(value)));
  return (
    <fieldset {...partOf(part.searchList)}>
      {field.choices.map((choice) => (
        <Checkbox
          key={choice}
          label={choice}
          isSelected={chosen.has(choice)}
          onChange={(on) => {
            const others = values.filter((value) => operandText(value) !== choice);
            onValues(on ? [...others, textOperand(choice)] : others);
          }}
        />
      ))}
    </fieldset>
  );
}

/** One choice of a fixed list. */
function ChoiceSelect({ field, values, onValues }: ValueInputProps) {
  const [first] = values;
  return (
    <SelectField
      label={t("search.bar.value")}
      choices={[
        { value: nothingPicked, label: t("search.bar.pick") },
        ...field.choices.map((choice) => ({ value: choice, label: choice })),
      ]}
      value={first === undefined ? nothingPicked : operandText(first)}
      onChange={(next) => {
        onValues(next === nothingPicked ? [] : [textOperand(next)]);
      }}
    />
  );
}

/** A text, a name or a number, typed: the value is only kept while it is one the field takes. */
function TypedValue({ field, values, onValues }: ValueInputProps) {
  const isNumber = field.kind === "number";
  const [first] = values;
  const [typed, setTyped] = useState(
    isNumber ? numberTyped(first) : first?.type === "text" ? first.text : "",
  );
  const me = first?.type === "me";
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
    <>
      <TextField
        label={t("search.bar.value")}
        value={me ? "" : typed}
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
      {field.kind === "person" ? (
        <Checkbox
          label={t("search.bar.me")}
          isSelected={me}
          onChange={(on) => {
            onValues(on ? [{ type: "me", span: nowhere }] : operand(typed));
          }}
        />
      ) : null}
    </>
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
          ...timeWords.map((each) => ({ value: each, label: timeWordText(each) })),
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
          value={typed}
          date={{ min: "0001-01-01", max: "9999-12-31" }}
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
  if (field.kind === "issue") {
    return (
      <IssueChoice language={props.language} values={props.values} onValues={props.onValues} />
    );
  }
  return <TypedValue {...props} />;
}
