import type { Expr, ReadableFields, Search, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { editingFor, type Editing } from "./editing.ts";
import { FilterList } from "./FilterList.tsx";
import { FilterWindows } from "./FilterWindows.tsx";
import { addFilter, changeFilter, filtersOf, removeFilter } from "../../shared/search-tree.ts";
import type { SearchBarState } from "./use-search-bar.ts";

export interface SimpleSearchProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  readonly bar: SearchBarState;
  /** The search to edit: the tree of the text in the address. */
  readonly search: Search;
}

/**
 * Simple search (QRY-01): each filter is a button that changes it, its ✕ takes it away, and
 * *+ Add filter* lists the fields the person may read. What has no picker is a dashed button with
 * its text, editable as text, so switching to Advanced search never loses anything (QRY-02).
 */
export function SimpleSearch({ language, fields, bar, search }: SimpleSearchProps) {
  const [editing, setEditing] = useState<Editing | undefined>();
  const close = (): void => {
    setEditing(undefined);
  };
  const apply = (filter: Expr): void => {
    const place = editing?.place;
    bar.change(
      place === undefined ? addFilter(search, filter) : changeFilter(search, place, filter),
    );
    close();
  };
  return (
    <>
      <FilterList
        language={language}
        fields={fields}
        filters={filtersOf(search)}
        onChange={(filter, index) => {
          setEditing(editingFor(fields, filter, { index }));
        }}
        onRemove={(index) => {
          bar.change(removeFilter(search, { index }));
        }}
        onAdd={(field) => {
          setEditing({ kind: "field", field });
        }}
        onAnyOf={() => {
          setEditing({ kind: "text", anyOf: true });
        }}
      />
      <FilterWindows
        language={language}
        fields={fields}
        editing={editing}
        onApply={apply}
        onClose={close}
      />
    </>
  );
}
