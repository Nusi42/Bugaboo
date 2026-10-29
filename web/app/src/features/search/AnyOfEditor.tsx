import { t } from "@bugaboo/messages";
import { AskDialog } from "@bugaboo/ui";
import type { Expr, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { joinedOr } from "../../shared/search-tree.ts";
import { AnyOfFilters } from "./AnyOfFilters.tsx";
import { editingFor, type Editing } from "./editing.ts";
import { FieldChooser } from "./FieldChooser.tsx";
import { FilterWindows } from "./FilterWindows.tsx";

export interface AnyOfEditorProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** The filters the window starts with; none for a new *Any of*. */
  readonly members: readonly Expr[];
  readonly isNew: boolean;
  readonly onApply: (filter: Expr) => void;
  readonly onClose: () => void;
}

/** A filter that is changed in its own window. `index` is its place among the filters; none for a new one. */
interface Inner {
  readonly editing: Editing;
  readonly index: number | undefined;
}

/** How many filters an *Any of* needs: one filter is just a filter. */
const fewest = 2;

/** The filters with `filter` in the place of the one at `index`, or at the end when there is no place. */
function withFilter(
  filters: readonly Expr[],
  index: number | undefined,
  filter: Expr,
): readonly Expr[] {
  return index === undefined
    ? [...filters, filter]
    : filters.map((each, at) => (at === index ? filter : each));
}

interface AnyOfDialogProps {
  readonly language: SearchLanguage;
  readonly filters: readonly Expr[];
  readonly isNew: boolean;
  readonly onChange: (filter: Expr, index: number) => void;
  readonly onRemove: (index: number) => void;
  readonly onAdd: () => void;
  readonly onApply: (filter: Expr) => void;
  readonly onClose: () => void;
}

/** The question: the filters inside, and what is still needed before the *Any of* can be added. */
function AnyOfDialog({
  language,
  filters,
  isNew,
  onChange,
  onRemove,
  onAdd,
  onApply,
  onClose,
}: AnyOfDialogProps) {
  const joined = filters.length < fewest ? null : joinedOr(filters);
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      title={t("search.bar.any-of")}
      text={t("search.bar.any-of-text")}
      confirmLabel={t(isNew ? "search.bar.any-of-confirm" : "search.bar.any-of-change")}
      cancelLabel={t("common.cancel")}
      confirmDisabled={joined === null}
      confirmHint={t("search.bar.any-of-need")}
      onConfirm={() => {
        if (joined !== null) {
          onApply(joined);
        }
      }}
    >
      <AnyOfFilters
        language={language}
        filters={filters}
        onChange={onChange}
        onRemove={onRemove}
        onAdd={onAdd}
      />
    </AskDialog>
  );
}

interface AnyOfWindowsProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** The list of fields is open. */
  readonly choosing: boolean;
  readonly onChoosing: (open: boolean) => void;
  readonly inner: Inner | null;
  readonly onInner: (inner: Inner | null) => void;
  /** A filter was made or changed in its window. */
  readonly onFilter: (filter: Expr) => void;
}

/** The windows over the *Any of*: the list of fields, then the window of the field picked or of the filter pressed. */
function AnyOfWindows({
  language,
  fields,
  choosing,
  onChoosing,
  inner,
  onInner,
  onFilter,
}: AnyOfWindowsProps) {
  const close = (): void => {
    onInner(null);
  };
  return (
    <>
      <FieldChooser
        isOpen={choosing}
        onOpenChange={onChoosing}
        fields={fields}
        title={t("search.bar.any-of-add")}
        onPick={(field) => {
          onInner({ editing: { kind: "field", field }, index: undefined });
        }}
      />
      <FilterWindows
        language={language}
        fields={fields}
        editing={inner?.editing}
        onApply={(filter) => {
          onFilter(filter);
          close();
        }}
        onClose={close}
      />
    </>
  );
}

/**
 * The window of an *Any of* (UX design section 4.3): the filters are inside, each a button like a
 * filter of Simple search, and *+ Add a filter* picks a field with the same list and the same
 * windows as *+ Add filter*. A result needs to fit at least one of them.
 */
export function AnyOfEditor({
  language,
  fields,
  members,
  isNew,
  onApply,
  onClose,
}: AnyOfEditorProps) {
  const [filters, setFilters] = useState<readonly Expr[]>(members);
  const [choosing, setChoosing] = useState(false);
  const [inner, setInner] = useState<Inner | null>(null);
  return (
    <>
      <AnyOfDialog
        language={language}
        filters={filters}
        isNew={isNew}
        onChange={(filter, index) => {
          setInner({ editing: editingFor(fields, filter, { index }), index });
        }}
        onRemove={(index) => {
          setFilters(filters.filter((_, at) => at !== index));
        }}
        onAdd={() => {
          setChoosing(true);
        }}
        onApply={onApply}
        onClose={onClose}
      />
      <AnyOfWindows
        language={language}
        fields={fields}
        choosing={choosing}
        onChoosing={setChoosing}
        inner={inner}
        onInner={setInner}
        onFilter={(filter) => {
          setFilters(withFilter(filters, inner?.index, filter));
        }}
      />
    </>
  );
}
