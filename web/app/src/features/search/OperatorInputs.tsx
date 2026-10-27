import { t } from "@bugaboo/messages";
import { SelectField, TextField } from "@bugaboo/ui";
import type { Field, SearchLanguage, Unit } from "@bugaboo/wasm-search";

import type { Draft } from "./filter-draft.ts";
import { ValueInputs } from "./ValueInputs.tsx";

const units: readonly Unit[] = ["day", "week", "month"];

const unitWords: Readonly<Record<Unit, () => string>> = {
  day: () => t("search.bar.unit-days"),
  week: () => t("search.bar.unit-weeks"),
  month: () => t("search.bar.unit-months"),
};

/** The number and the unit of *in the last 7 days*. */
function LastInputs({
  draft,
  onDraft,
}: {
  readonly draft: Draft;
  readonly onDraft: (draft: Draft) => void;
}) {
  return (
    <>
      <TextField
        label={t("search.bar.count")}
        value={String(draft.count)}
        onChange={(count) => {
          onDraft({ ...draft, count: Math.max(0, Math.trunc(Number(count)) || 0) });
        }}
      />
      <SelectField
        label={t("search.bar.unit")}
        choices={units.map((unit) => ({ value: unit, label: unitWords[unit]() }))}
        value={draft.unit}
        onChange={(unit) => {
          onDraft({ ...draft, unit });
        }}
      />
    </>
  );
}

/** The inputs that depend on the operator: the values, the number of days, or a text. */
export function OperatorInputs(props: {
  readonly field: Field;
  readonly language: SearchLanguage;
  readonly draft: Draft;
  readonly onDraft: (draft: Draft) => void;
}) {
  const { field, language, draft, onDraft } = props;
  const { operator } = draft;
  if (operator === "is_empty" || operator === "is_not_empty") {
    return null;
  }
  if (operator === "in_last") {
    return <LastInputs draft={draft} onDraft={onDraft} />;
  }
  if (operator === "contains" || operator === "member_of" || operator === "within") {
    return (
      <TextField
        label={t("search.bar.value")}
        value={draft.text}
        onChange={(text) => {
          onDraft({ ...draft, text });
        }}
      />
    );
  }
  const many = operator === "in";
  return (
    <ValueInputs
      field={field}
      language={language}
      values={draft.values}
      many={many}
      onValues={(values) => {
        onDraft({ ...draft, values });
      }}
    />
  );
}
