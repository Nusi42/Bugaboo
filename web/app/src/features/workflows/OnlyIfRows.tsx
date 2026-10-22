import { testWords, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, IconButton, iconName, SelectField, partOf, part } from "@bugaboo/ui";

import {
  newCondition,
  type ChangeRules,
  type RuleCondition,
  type RuleFieldChoice,
} from "./rules-model.ts";

export interface OnlyIfRowsProps {
  /** The fields a condition can name, by their names. */
  readonly fields: readonly RuleFieldChoice[];
  readonly rules: ChangeRules;
  readonly onChange: (rules: ChangeRules) => void;
}

/** The tests of a condition, by the words people read. */
const testLabels: Record<Schema<"Test">, string> = {
  filled: t("workflows.test-filled"),
  empty: t("workflows.test-empty"),
};

/** The tests of a condition in the order the server lists them. */
const testChoices = testWords.map((word) => ({ value: word, label: testLabels[word] }));

/** One condition: the field, what it must be like, and a way to take it away. */
function ConditionRow({
  condition,
  fields,
  onChange,
  onRemove,
}: {
  readonly condition: RuleCondition;
  readonly fields: readonly RuleFieldChoice[];
  readonly onChange: (change: Partial<RuleCondition>) => void;
  readonly onRemove: () => void;
}) {
  const named = fields.find((each) => each.id === condition.field)?.name ?? condition.field;
  return (
    <div {...partOf(part.workflowCondition)}>
      <SelectField
        label={t("workflows.only-if-field")}
        choices={fields.map((each) => ({ value: each.id, label: each.name }))}
        value={condition.field}
        onChange={(field) => {
          onChange({ field });
        }}
      />
      <SelectField
        label={t("workflows.only-if-test")}
        choices={testChoices}
        value={condition.test}
        onChange={(test) => {
          onChange({ test });
        }}
      />
      <IconButton
        icon={iconName.close}
        label={t("workflows.remove-condition", { field: named })}
        onPress={onRemove}
      />
    </div>
  );
}

/** *Only if …*: every condition is a field and what it must be like, in the same words as *Must be filled in*. */
export function OnlyIfRows({ fields, rules, onChange }: OnlyIfRowsProps) {
  const first = fields[0];
  return (
    <fieldset {...partOf(part.workflowRules)}>
      <legend {...partOf(part.workflowRulesLegend)}>{t("workflows.only-if")}</legend>
      <p {...partOf(part.muted)}>{t("workflows.only-if-help")}</p>
      {rules.onlyIf.map((condition) => (
        <ConditionRow
          key={condition.id}
          condition={condition}
          fields={fields}
          onChange={(change) => {
            onChange({
              ...rules,
              onlyIf: rules.onlyIf.map((each) =>
                each.id === condition.id ? { ...each, ...change } : each,
              ),
            });
          }}
          onRemove={() => {
            onChange({ ...rules, onlyIf: rules.onlyIf.filter((each) => each.id !== condition.id) });
          }}
        />
      ))}
      {first === undefined ? null : (
        <div>
          <Button
            onPress={() => {
              onChange({ ...rules, onlyIf: [...rules.onlyIf, newCondition(first.id)] });
            }}
          >
            {t("workflows.add-condition")}
          </Button>
        </div>
      )}
    </fieldset>
  );
}
