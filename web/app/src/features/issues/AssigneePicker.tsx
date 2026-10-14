import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  Overlay,
  PickerHint,
  PickerRow,
  SearchPicker,
  part,
  partOf,
} from "@bugaboo/ui";

import type { NamedPerson } from "../../shared/person.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { peopleOf, useAssignablePeople } from "./use-assignable-people.ts";

export interface AssigneePickerProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** The project of the issue: its short name. */
  readonly project: string;
  /** The issue (its ID): who reported it decides who may see it, and so who is offered. */
  readonly issue?: string | undefined;
  /** The ID of the type of the issue that is being made: a level per issue type then decides. */
  readonly issueType?: string | undefined;
  /** The people it is assigned to now. */
  readonly assigned: readonly NamedPerson[];
  /** The people it is assigned to from now on: all of them. */
  readonly onChange: (people: readonly NamedPerson[]) => void;
}

interface RowProps {
  readonly person: NamedPerson;
  readonly assigned: boolean;
  readonly onAssign: () => void;
  readonly onUnassign: () => void;
}

/** A person of the project: a button to assign them, or the word that they are, and a button to take them away. */
function Row({ person, assigned, onAssign, onUnassign }: RowProps) {
  return (
    <PickerRow label={person.name}>
      {assigned ? (
        <>
          <PickerHint>{t("issues.assigned")}</PickerHint>
          <Button
            aria-label={t("issues.unassign-name", { name: person.name })}
            onPress={onUnassign}
          >
            {t("issues.unassign")}
          </Button>
        </>
      ) : (
        <Button aria-label={t("issues.assign-name", { name: person.name })} onPress={onAssign}>
          {t("issues.assign")}
        </Button>
      )}
    </PickerRow>
  );
}

interface InsideProps {
  readonly project: string;
  readonly issue?: string | undefined;
  readonly issueType?: string | undefined;
  readonly assigned: readonly NamedPerson[];
  readonly onChange: (people: readonly NamedPerson[]) => void;
  readonly onClose: () => void;
}

/** The inside of the dialog, drawn only while it is open so nothing is asked before. */
function Inside({ project, issue, issueType, assigned, onChange, onClose }: InsideProps) {
  const { words, frame } = usePickerFrame();
  const found = useAssignablePeople(project, words, { issue, issueType });
  const people = peopleOf(found.data?.pages);
  return (
    <div>
      <SearchPicker
        legend={t("issues.assign-title")}
        searchLabel={t("issues.assign-search")}
        listLabel={t("issues.assign-list")}
        {...frame}
        {...pickerStatus(found, people.length, {
          nothing: t("issues.assign-nothing"),
          more: t("issues.assign-more"),
        })}
      >
        {people.map((person) => (
          <Row
            key={person.id}
            person={person}
            assigned={assigned.some((known) => known.id === person.id)}
            onAssign={() => {
              onChange([...assigned, person]);
            }}
            onUnassign={() => {
              onChange(assigned.filter((known) => known.id !== person.id));
            }}
          />
        ))}
      </SearchPicker>
      {words === "" && people.length <= 1 && !found.isPending ? (
        <p {...partOf(part.muted)}>{t("issues.assign-only-you")}</p>
      ) : null}
      <Button look={buttonLook.primary} onPress={onClose}>
        {t("common.done")}
      </Button>
    </div>
  );
}

/**
 * Choosing who an issue is assigned to (UX §4.4): the people of the project, found by name, each
 * with a button to assign or take away; every press saves at once. It is a dialog (a short task).
 */
export function AssigneePicker({
  isOpen,
  onClose,
  project,
  issue,
  issueType,
  assigned,
  onChange,
}: AssigneePickerProps) {
  return (
    <Overlay
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={t("issues.assign-title")}
    >
      {isOpen ? (
        <Inside
          project={project}
          issue={issue}
          issueType={issueType}
          assigned={assigned}
          onChange={onChange}
          onClose={onClose}
        />
      ) : null}
    </Overlay>
  );
}
