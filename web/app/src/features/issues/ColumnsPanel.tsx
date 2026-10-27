import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf } from "@bugaboo/ui";

import { ColumnList } from "./ColumnList.tsx";
import type { ColumnChoice } from "./columns-choice.ts";

export interface ColumnsPanelProps {
  /** The columns now, in order. */
  readonly columns: readonly string[];
  /** The fields the person may read. */
  readonly readable: readonly ColumnChoice[];
  readonly onChange: (columns: readonly string[]) => void;
  /** Back to the columns, and the widths, the project's list or the saved search has. */
  readonly onReset: () => void;
  readonly onClose: () => void;
}

/**
 * *Customize › Columns* (UX section 4.2): every field the person may read as a checklist; the
 * ticked ones come first in the order of the columns, moved with *Up* and *Down*. *Back to the
 * project's columns* undoes the changes.
 */
export function ColumnsPanel({ columns, readable, onChange, onReset, onClose }: ColumnsPanelProps) {
  return (
    <aside {...partOf(part.listPanel)} aria-label={t("issues.customize")}>
      <div {...partOf(part.listPanelHead)}>
        <h2 {...partOf(part.listPanelTitle)}>{t("issues.customize-columns")}</h2>
        <Button look={buttonLook.quiet} onPress={onClose}>
          {t("common.close")}
        </Button>
      </div>
      <ul {...partOf(part.listPanelItems)}>
        <ColumnList columns={columns} readable={readable} onChange={onChange} />
      </ul>
      <Button onPress={onReset}>{t("issues.columns-reset")}</Button>
    </aside>
  );
}
