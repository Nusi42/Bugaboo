import type { Expr, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";

import type { Editing } from "./editing.ts";
import { AnyOfEditor } from "./AnyOfEditor.tsx";
import { FilterEditor } from "./FilterEditor.tsx";
import { sourceOf } from "./filter-text.ts";
import { TextFilterEditor } from "./TextFilterEditor.tsx";

export interface FilterWindowsProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** What is open; nothing while no window is. */
  readonly editing: Editing | undefined;
  readonly onApply: (filter: Expr) => void;
  readonly onClose: () => void;
}

/** The window of the filter that is open: its picker, or its text. */
export function FilterWindows({ language, fields, editing, onApply, onClose }: FilterWindowsProps) {
  if (editing?.kind === "field") {
    return (
      <FilterEditor
        field={editing.field}
        {...(editing.filter === undefined ? {} : { filter: editing.filter })}
        language={language}
        fields={fields}
        onApply={onApply}
        onClose={onClose}
      />
    );
  }
  if (editing?.kind === "any-of") {
    return (
      <AnyOfEditor
        language={language}
        fields={fields}
        members={editing.members}
        isNew={editing.place === undefined}
        onApply={onApply}
        onClose={onClose}
      />
    );
  }
  return editing?.kind === "text" ? (
    <TextFilterEditor
      language={language}
      fields={fields}
      text={editing.filter === undefined ? "" : sourceOf(language, editing.filter)}
      isNew={editing.place === undefined}
      onApply={onApply}
      onClose={onClose}
    />
  ) : null;
}
