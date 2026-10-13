import { partOf, part } from "@bugaboo/ui";

import type { OwnField } from "./fields.ts";
import { ValueEditor, type Sent } from "./ValueEditor.tsx";
import { ValueView } from "./ValueView.tsx";

export interface FieldLineProps {
  readonly field: OwnField;
  /** The project of the item: its short name. */
  readonly project: string | undefined;
  /** The issue, when the item is one: its ID. */
  readonly issue?: string | undefined;
  readonly value: unknown;
  readonly editable: boolean;
  readonly onCommit: (field: OwnField, value: Sent) => void;
}

/** One field: its name and value, or its editor where the person may change it. */
export function FieldLine({ field, project, issue, value, editable, onCommit }: FieldLineProps) {
  if (!editable) {
    return (
      <div {...partOf(part.stack)}>
        <p {...partOf(part.fieldLabel)}>{field.name}</p>
        <ValueView field={field} value={value} />
      </div>
    );
  }
  return (
    <ValueEditor
      field={field}
      project={project}
      issue={issue}
      value={value}
      onCommit={(sent) => {
        onCommit(field, sent);
      }}
    />
  );
}
