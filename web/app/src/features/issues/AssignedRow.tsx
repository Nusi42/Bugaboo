import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part, type ChangeStatus } from "@bugaboo/ui";
import { useState } from "react";

import { namedPerson, personName } from "../../shared/person.ts";
import { useShortcuts } from "../../shared/use-shortcuts.ts";
import { AssignedPicker } from "./AssignedPicker.tsx";
import { DetailRow } from "./DetailRow.tsx";
import type { IssueValues } from "./issue-values.ts";
import { readOnlyReason } from "./read-only.ts";
import type { IssueFieldMask, IssueView } from "./slots.ts";

export interface AssignedRowProps {
  readonly issue: IssueView;
  readonly mask: IssueFieldMask;
  readonly status: ChangeStatus | undefined;
  readonly onChange: (values: IssueValues) => void;
}

/** The shortcut that opens the picker, while the person may change *Assigned to*. */
function useAssignShortcut(canChange: boolean, open: () => void): void {
  useShortcuts(canChange ? { assign: open } : {});
}

/** The button that opens the picker: *Assign someone* while nobody is, *Change* after that. */
export function ChangeAssigned({
  name,
  empty,
  onPress,
}: {
  readonly name: string;
  readonly empty: boolean;
  readonly onPress: () => void;
}) {
  return (
    <Button
      look={buttonLook.quiet}
      aria-label={empty ? t("issues.assign-someone") : t("issues.change-field", { field: name })}
      onPress={onPress}
    >
      {empty ? t("issues.assign-someone") : t("issues.change")}
    </Button>
  );
}

/** One line for each person assigned, as high as a control. */
function PeopleLines({ people }: { readonly people: NonNullable<IssueView["assigned_to"]> }) {
  return people.map((person) => (
    <span key={namedPerson(person)?.id ?? personName(person)} {...partOf(part.startColumnLine)}>
      {personName(person)}
    </span>
  ));
}

/** *Assigned to*: the people, and the way to change them or to add the first one. */
export function AssignedRow({ issue, mask, status, onChange }: AssignedRowProps) {
  const [picking, setPicking] = useState(false);
  const people = issue.assigned_to ?? [];
  const name = t("issue-field.assigned-to");
  const canChange = mask.changes("assigned-to");
  useAssignShortcut(canChange, () => {
    setPicking(true);
  });
  if (!canChange && people.length === 0) {
    return null;
  }
  const named = people.flatMap((person) => namedPerson(person) ?? []);
  return (
    <>
      <DetailRow
        name={name}
        reason={canChange ? undefined : readOnlyReason(name)}
        status={canChange ? status : undefined}
      >
        <div {...partOf(part.startColumn)}>
          <PeopleLines people={people} />
          {canChange ? (
            <ChangeAssigned
              name={name}
              empty={people.length === 0}
              onPress={() => {
                setPicking(true);
              }}
            />
          ) : null}
        </div>
      </DetailRow>
      <AssignedPicker
        isOpen={picking}
        onClose={() => {
          setPicking(false);
        }}
        project={issue.project?.short_name ?? ""}
        issue={issue.id ?? undefined}
        assigned={named}
        onChange={(next) => {
          onChange({ assignedTo: next });
        }}
      />
    </>
  );
}
