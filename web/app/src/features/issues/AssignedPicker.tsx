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

export interface AssignedPickerProps {
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
  /** What the picker is for, where it is not *Assigned to*: its title and the name of its button. */
  readonly words?: PickerWords;
}

/** The title of the picker and the button that picks a person, by what the picker is for. */
export interface PickerWords {
  readonly title: string;
  /** The words of the button of a person, and its name for screen readers. */
  readonly pick: string;
  readonly pickName: (name: string) => string;
}

/** The words of the picker of *Assigned to* (UX §4.4). */
function assignWords(): PickerWords {
  return {
    title: t("issues.assign-title"),
    pick: t("issues.assign"),
    pickName: (name) => t("issues.assign-name", { name }),
  };
}

interface RowProps {
  readonly person: NamedPerson;
  readonly words: PickerWords;
  readonly assigned: boolean;
  readonly onAssign: () => void;
  readonly onUnassign: () => void;
}

/** A person of the project: a button to assign them, or the word that they are, and a button to take them away. */
function Row({ person, words, assigned, onAssign, onUnassign }: RowProps) {
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
        <Button aria-label={words.pickName(person.name)} onPress={onAssign}>
          {words.pick}
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
  readonly words: PickerWords;
}

/** The inside of the dialog, drawn only while it is open so nothing is asked before. */
function Inside({
  project,
  issue,
  issueType,
  assigned,
  onChange,
  onClose,
  words: said,
}: InsideProps) {
  const { words, frame } = usePickerFrame();
  const found = useAssignablePeople(project, words, { issue, issueType });
  const people = peopleOf(found.data?.pages);
  return (
    <div {...partOf(part.assignedPicker)}>
      <h2 {...partOf(part.confirmTitle)}>{said.title}</h2>
      <SearchPicker
        legend={t("issues.assign-list")}
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
            words={said}
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
export function AssignedPicker({
  isOpen,
  onClose,
  project,
  issue,
  issueType,
  assigned,
  onChange,
  words = assignWords(),
}: AssignedPickerProps) {
  return (
    <Overlay
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
      label={words.title}
    >
      {isOpen ? (
        <Inside
          project={project}
          issue={issue}
          issueType={issueType}
          assigned={assigned}
          onChange={onChange}
          onClose={onClose}
          words={words}
        />
      ) : null}
    </Overlay>
  );
}
