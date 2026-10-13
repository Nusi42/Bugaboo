import { useFields } from "./fields.ts";
import { ValueEditor, type Sent } from "./ValueEditor.tsx";

export interface OwnFieldInputProps {
  /** The ID of the own field. */
  readonly field: string;
  /** The project the issue is made in: its short name. */
  readonly project: string;
  /** The ID of the type of the issue that is being made. */
  readonly issueType: string | undefined;
  readonly value: unknown;
  /** Hands on the value as the API takes it, at every change. */
  readonly onChange: (value: Sent) => void;
}

/**
 * An own field on the form of a new issue (CF-03, CFG-01): its editor, with no *Save* of its own,
 * because the form is sent as a whole. The people it can point to are the ones the person may
 * name for an issue of this type.
 */
export function OwnFieldInput({ field, project, issueType, value, onChange }: OwnFieldInputProps) {
  const found = useFields({ project }).data?.fields.find((each) => each.id === field);
  return found === undefined ? null : (
    <ValueEditor
      field={found}
      project={project}
      issueType={issueType}
      value={value}
      live
      onCommit={onChange}
    />
  );
}
