import { NameValueList, NameValueRow, type ChangeStatus } from "@bugaboo/ui";

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
  /** What the last change of this field came to: *Saved* and *Undo*, where it happened. */
  readonly status?: ChangeStatus | undefined;
  readonly onCommit: (field: OwnField, value: Sent) => void;
}

/**
 * One field of the Details as a name-and-value row (UX §4.4): the name at the left, the value or its
 * editor at the right. The editor saves in place, with no *Save* button, and the row says *Saved*
 * with *Undo*.
 */
export function FieldLine({
  field,
  project,
  issue,
  value,
  editable,
  status,
  onCommit,
}: FieldLineProps) {
  return (
    <NameValueList>
      <NameValueRow name={field.name} {...(editable && status !== undefined ? { status } : {})}>
        {editable ? (
          <ValueEditor
            field={field}
            project={project}
            issue={issue}
            value={value}
            inRow
            onCommit={(sent) => {
              onCommit(field, sent);
            }}
          />
        ) : (
          <ValueView field={field} value={value} />
        )}
      </NameValueRow>
    </NameValueList>
  );
}
