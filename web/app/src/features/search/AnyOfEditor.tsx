import { t } from "@bugaboo/messages";
import { AskDialog } from "@bugaboo/ui";
import type { Expr, ReadableFields, SearchLanguage } from "@bugaboo/wasm-search";
import { useState } from "react";

import { joinedOr } from "../../shared/search-tree.ts";
import { AnyOfConditions } from "./AnyOfConditions.tsx";
import { editingFor, type Editing } from "./editing.ts";
import { FieldChooser } from "./FieldChooser.tsx";
import { FilterWindows } from "./FilterWindows.tsx";

export interface AnyOfEditorProps {
  readonly language: SearchLanguage;
  readonly fields: ReadableFields;
  /** The conditions the window starts with; none for a new *Any of*. */
  readonly members: readonly Expr[];
  readonly isNew: boolean;
  readonly onApply: (filter: Expr) => void;
  readonly onClose: () => void;
}

/** A condition that is changed in its own window. `index` is its place among the conditions; none for a new one. */
interface Inner {
  readonly editing: Editing;
  readonly index: number | undefined;
}

/** How many conditions an *Any of* needs: one condition is just a filter. */
const fewest = 2;

/** The conditions with `condition` in the place of the one at `index`, or at the end when there is no place. */
function withCondition(
  conditions: readonly Expr[],
  index: number | undefined,
  condition: Expr,
): readonly Expr[] {
  return index === undefined
    ? [...conditions, condition]
    : conditions.map((each, at) => (at === index ? condition : each));
}

interface AnyOfDialogProps {
  readonly language: SearchLanguage;
  readonly conditions: readonly Expr[];
  readonly isNew: boolean;
  readonly onChange: (condition: Expr, index: number) => void;
  readonly onRemove: (index: number) => void;
  readonly onAdd: () => void;
  readonly onApply: (filter: Expr) => void;
  readonly onClose: () => void;
}

/** The question: the conditions inside, and what is still needed before the *Any of* can be added. */
function AnyOfDialog({
  language,
  conditions,
  isNew,
  onChange,
  onRemove,
  onAdd,
  onApply,
  onClose,
}: AnyOfDialogProps) {
  const joined = conditions.length < fewest ? null : joinedOr(conditions);
  const confirm = isNew ? "search.bar.add-filter-confirm" : "search.bar.change-filter-confirm";
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
      confirmLabel={t(confirm)}
      cancelLabel={t("common.cancel")}
      confirmDisabled={joined === null}
      confirmHint={t("search.bar.any-of-need")}
      onConfirm={() => {
        if (joined !== null) {
          onApply(joined);
        }
      }}
    >
      <AnyOfConditions
        language={language}
        conditions={conditions}
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
  /** A condition was made or changed in its window. */
  readonly onCondition: (condition: Expr) => void;
}

/** The windows over the *Any of*: the list of fields, then the window of the field picked or of the filter pressed. */
function AnyOfWindows({
  language,
  fields,
  choosing,
  onChoosing,
  inner,
  onInner,
  onCondition,
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
        onApply={(condition) => {
          onCondition(condition);
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
  const [conditions, setConditions] = useState<readonly Expr[]>(members);
  const [choosing, setChoosing] = useState(false);
  const [inner, setInner] = useState<Inner | null>(null);
  return (
    <>
      <AnyOfDialog
        language={language}
        conditions={conditions}
        isNew={isNew}
        onChange={(condition, index) => {
          setInner({ editing: editingFor(fields, condition, { index }), index });
        }}
        onRemove={(index) => {
          setConditions(conditions.filter((_, at) => at !== index));
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
        onCondition={(condition) => {
          setConditions(withCondition(conditions, inner?.index, condition));
        }}
      />
    </>
  );
}
