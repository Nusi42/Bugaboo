import { t } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, SelectField } from "@bugaboo/ui";
import type { Expr, Field, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { draftOf, filterOf, type Draft } from "./filter-draft.ts";
import { useFieldName } from "./field-name-context.ts";
import { mistakeText } from "./mistake-text.ts";
import { operatorLabel } from "./operator-words.ts";
import { nowhere, type FieldFilter } from "../../shared/search-tree.ts";
import { OperatorInputs } from "./OperatorInputs.tsx";

export interface FilterEditorProps {
  readonly field: Field;
  /** The filter that is changed; none for a new one. */
  readonly filter?: FieldFilter;
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** What the filter is checked against and then put into the search. */
  readonly onApply: (filter: Expr) => void;
  readonly onClose: () => void;
}

/** The operators the window offers: a list of values is for fields that have a list. */
function operatorsOf(field: Field) {
  return field.operators.filter((operator) => operator !== "in" || field.choices.length > 0);
}

/** The first mistake the checker finds in the filter as it would be written, in plain words. */
function problemOf(
  language: SearchLanguage,
  fields: ReadableFields,
  filter: Expr | null,
): string | undefined {
  if (filter === null) {
    return undefined;
  }
  const text = language.print({ filter, sort: [], span: nowhere });
  const [mistake] = language.check(text, fields).mistakes;
  return mistake === undefined ? undefined : mistakeText(mistake);
}

/** The list of the operators the field allows. */
function OperatorSelect({
  field,
  draft,
  onDraft,
}: {
  readonly field: Field;
  readonly draft: Draft;
  readonly onDraft: (draft: Draft) => void;
}) {
  return (
    <SelectField
      label={t("search.bar.operator")}
      choices={operatorsOf(field).map((operator) => ({
        value: operator,
        label: operatorLabel(operator, field.kind),
      }))}
      value={draft.operator}
      onChange={(operator) => {
        onDraft({ ...draft, operator });
      }}
    />
  );
}

/**
 * The window of one filter (UX design section 4.3): the operator and the value, in the way the
 * kind of the field is picked. The filter is checked as the search would be, so a mistake is told
 * before it is put into the search.
 */
export function FilterEditor({
  field,
  filter,
  language,
  fields,
  onApply,
  onClose,
}: FilterEditorProps) {
  const [draft, setDraft] = useState<Draft>(() => draftOf(field, filter));
  const made = filterOf(field, draft);
  const problem = problemOf(language, fields, made);
  const name = useFieldName()(field.word);
  const isNew = filter === undefined;
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={name}
      text={
        isNew
          ? t("search.bar.add-filter-text", { field: name })
          : t("search.bar.change-filter-text", { field: name })
      }
      confirmLabel={
        isNew ? t("search.bar.add-filter-confirm") : t("search.bar.change-filter-confirm")
      }
      cancelLabel={t("common.cancel")}
      confirmDisabled={made === null || problem !== undefined}
      onConfirm={() => {
        if (made !== null) {
          onApply(made);
        }
      }}
    >
      <OperatorSelect field={field} draft={draft} onDraft={setDraft} />
      <OperatorInputs field={field} language={language} draft={draft} onDraft={setDraft} />
      {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
    </AskDialog>
  );
}
