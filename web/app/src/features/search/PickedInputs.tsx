import { t } from "@bugaboo/messages";
import { Checkbox, SelectField, part, partOf } from "@bugaboo/ui";

import { useChoiceWords } from "./field-name-context.ts";
import { operandText, shownChoices, shownWord } from "./filter-text.ts";
import { nothingPicked, textOperand, type ValueInputProps } from "./filter-draft.ts";
import { NameChoice } from "./NameChoice.tsx";
import { nowhere } from "../../shared/search-tree.ts";

/** Several choices of a fixed list, ticked (`status in (Open, Done)`). */
export function ChoiceBoxes({ field, values, onValues }: ValueInputProps) {
  const choiceOf = useChoiceWords();
  const chosen = new Set(values.map((value) => shownWord(field, choiceOf, operandText(value))));
  return (
    <fieldset {...partOf(part.searchList)}>
      {shownChoices(field, choiceOf).map((choice) => (
        <Checkbox
          key={choice}
          label={choice}
          isSelected={chosen.has(choice)}
          onChange={(on) => {
            const others = values.filter(
              (value) => shownWord(field, choiceOf, operandText(value)) !== choice,
            );
            onValues(on ? [...others, textOperand(choice)] : others);
          }}
        />
      ))}
    </fieldset>
  );
}

/** One choice of a fixed list. */
export function ChoiceSelect({ field, values, onValues }: ValueInputProps) {
  const choiceOf = useChoiceWords();
  const [first] = values;
  return (
    <SelectField
      label={t("search.bar.value")}
      choices={[
        { value: nothingPicked, label: t("search.bar.pick") },
        ...shownChoices(field, choiceOf).map((choice) => ({
          value: choice,
          label: choice,
        })),
      ]}
      value={first === undefined ? nothingPicked : shownWord(field, choiceOf, operandText(first))}
      onChange={(next) => {
        onValues(next === nothingPicked ? [] : [textOperand(next)]);
      }}
    />
  );
}

/** A person or a project, found by name; for a person, the one who searches may be chosen as *me*. */
export function NamedValue({ field, language, values, onValues }: ValueInputProps) {
  const [first] = values;
  return (
    <NameChoice
      language={language}
      what={field.kind === "person" ? "people" : "projects"}
      chosen={first?.type === "me" ? t("search.bar.me") : first?.type === "text" ? first.text : ""}
      onChoose={(name) => {
        onValues([textOperand(name)]);
      }}
      {...(field.kind === "person"
        ? {
            onChooseMe: () => {
              onValues([{ type: "me", span: nowhere }]);
            },
          }
        : {})}
    />
  );
}
