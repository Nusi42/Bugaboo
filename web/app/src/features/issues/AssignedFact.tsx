import { t } from "@bugaboo/messages";
import { Button, buttonLook } from "@bugaboo/ui";
import { useState } from "react";

import { namedPerson, personName } from "../../shared/person.ts";
import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { AssigneePicker } from "./AssigneePicker.tsx";
import { Fact } from "./Fact.tsx";
import type { IssueValues } from "./issue-values.ts";
import { readOnlyReason } from "./read-only.ts";
import type { IssueFieldMask, IssueView } from "./slots.ts";

export interface AssignedFactProps {
  readonly issue: IssueView;
  readonly mask: IssueFieldMask;
  readonly onChange: (values: IssueValues) => void;
}

/** *Assigned to*: the people, and the way to change them or to add the first one. */
export function AssignedFact({ issue, mask, onChange }: AssignedFactProps) {
  const [picking, setPicking] = useState(false);
  const people = issue.assigned_to ?? [];
  const name = t("issue-field.assigned-to");
  const canChange = mask.changes("assigned-to");
  useShortcuts(
    canChange
      ? {
          assign: () => {
            setPicking(true);
          },
        }
      : {},
  );
  if (!canChange && people.length === 0) {
    return null;
  }
  const named = people.flatMap((person) => namedPerson(person) ?? []);
  return (
    <>
      <Fact name={name} reason={canChange ? undefined : readOnlyReason(name)}>
        {people.map((person) => (
          <span key={namedPerson(person)?.id ?? personName(person)}>{personName(person)}</span>
        ))}
        {canChange ? (
          <Button
            look={buttonLook.quiet}
            aria-label={people.length === 0 ? t("issues.add-field", { field: name }) : name}
            onPress={() => {
              setPicking(true);
            }}
          >
            {people.length === 0 ? t("issues.add-field", { field: name }) : t("issues.change")}
          </Button>
        ) : null}
      </Fact>
      <AssigneePicker
        isOpen={picking}
        onClose={() => {
          setPicking(false);
        }}
        project={issue.project?.short_name ?? ""}
        assigned={named}
        onChange={(next) => {
          onChange({ assignedTo: next });
        }}
      />
    </>
  );
}
