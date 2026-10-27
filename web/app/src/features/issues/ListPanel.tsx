import { isDrawnColumn } from "../../shared/list-columns.ts";
import type { ListView } from "../../shared/list-view.ts";
import type { SearchFieldsAnswer } from "../search/index.ts";
import { ColumnsPanel } from "./ColumnsPanel.tsx";

export interface ListPanelProps {
  readonly view: ListView;
  readonly fields: SearchFieldsAnswer;
  readonly defaults: ListView;
  readonly onView: (view: ListView) => void;
  readonly onClose: () => void;
}

/** Customize, then Columns: the fields the person may read, in the order the server lists them. */
export function ListPanel({ view, fields, defaults, onView, onClose }: ListPanelProps) {
  const known = new Map(fields.fields.fields.map((field) => [field.word, field]));
  return (
    <ColumnsPanel
      columns={view.columns}
      readable={Object.keys(fields.names)
        .filter((word) => isDrawnColumn(word, known.get(word)))
        .map((word) => ({ word, name: fields.names[word] ?? word }))}
      onReset={() => {
        onView({ ...view, columns: defaults.columns, widths: defaults.widths });
      }}
      onChange={(columns) => {
        onView({ ...view, columns });
      }}
      onClose={onClose}
    />
  );
}
