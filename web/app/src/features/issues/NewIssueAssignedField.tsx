import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { AssignedPicker } from "./AssignedPicker.tsx";
import { ChangeAssigned } from "./AssignedRow.tsx";
import type { NewIssueValues } from "./new-issue-form.ts";

export interface NewIssueAssignedFieldProps {
  readonly values: NewIssueValues;
  /** The project it is made in: its short name. */
  readonly project: string;
  readonly onChange: (values: NewIssueValues) => void;
}

/** *Assigned to* on the form: the people chosen so far, and the button that opens the picker. */
export function NewIssueAssignedField({ values, project, onChange }: NewIssueAssignedFieldProps) {
  const [picking, setPicking] = useState(false);
  const name = t("issue-field.assigned-to");
  return (
    <div>
      <p {...partOf(part.fieldLabel)}>{name}</p>
      <div {...partOf(part.startColumn)}>
        {values.assignedTo.map((person) => (
          <span key={person.id}>{person.name}</span>
        ))}
        <ChangeAssigned
          name={name}
          empty={values.assignedTo.length === 0}
          onPress={() => {
            setPicking(true);
          }}
        />
      </div>
      <AssignedPicker
        isOpen={picking}
        onClose={() => {
          setPicking(false);
        }}
        project={project}
        issueType={values.issueType === "" ? undefined : values.issueType}
        assigned={values.assignedTo}
        onChange={(assignedTo) => {
          onChange({ ...values, assignedTo });
        }}
      />
    </div>
  );
}
