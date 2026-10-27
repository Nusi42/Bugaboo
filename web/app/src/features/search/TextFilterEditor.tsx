import { t, textByKey } from "@bugaboo/messages";
import { Alert, alertTone, AskDialog, fieldKind, TextField } from "@bugaboo/ui";
import type { Expr, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

export interface TextFilterEditorProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** The text the window starts with: the text of the filter, or nothing for a new one. */
  readonly text: string;
  /** What the filter is: an *Any of* or a filter with no picker. */
  readonly anyOf: boolean;
  readonly isNew: boolean;
  readonly onApply: (filter: Expr) => void;
  readonly onClose: () => void;
}

/**
 * The window for what Simple search has no picker for: the filter as text, checked as it is typed.
 * An *Any of* is the same: the conditions with `or` between them (UX design section 4.3).
 */
export function TextFilterEditor({
  language,
  fields,
  text,
  anyOf,
  isNew,
  onApply,
  onClose,
}: TextFilterEditorProps) {
  const [typed, setTyped] = useState(text);
  const answer = language.check(typed, fields);
  const [mistake] = answer.mistakes;
  const filter = answer.search?.filter ?? null;
  // Only filters: `sort by` is not one, and a text with nothing in it is no filter.
  const usable = mistake === undefined && filter !== null && answer.search?.sort.length === 0;
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={anyOf ? t("search.bar.any-of") : t("search.bar.text-filter")}
      text={anyOf ? t("search.bar.any-of-text") : t("search.bar.text-filter-text")}
      confirmLabel={t(isNew ? "search.bar.add-filter-confirm" : "search.bar.change-filter-confirm")}
      cancelLabel={t("common.cancel")}
      confirmDisabled={!usable}
      onConfirm={() => {
        if (filter !== null) {
          onApply(filter);
        }
      }}
    >
      <TextField
        label={t("search.bar.filter-as-text")}
        kind={fieldKind.multiline}
        value={typed}
        onChange={setTyped}
      />
      {mistake === undefined ? null : (
        <Alert tone={alertTone.danger}>{textByKey(mistake.key, mistake.args)}</Alert>
      )}
    </AskDialog>
  );
}
