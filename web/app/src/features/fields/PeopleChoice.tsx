import { t } from "@bugaboo/messages";
import { Button, PickerHint, PickerRow, SearchPicker } from "@bugaboo/ui";

import type { NamedPerson } from "../../shared/person.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { peopleOf, useAssignablePeople } from "../issues/index.ts";
import { ChoiceDialog } from "./ChoiceDialog.tsx";
import type { OwnField } from "./fields.ts";
import { type Picked, picksOf } from "../../shared/reference.ts";

export interface PeopleChoiceProps {
  readonly field: OwnField;
  /** The value now, as the server showed it. */
  readonly value: unknown;
  /** The project of the item (its short name): the people of the project are offered. */
  readonly project: string;
  /** The issue (its ID), when the item is one: the people who may see it are offered. */
  readonly issue?: string | undefined;
  /** The ID of the type of the issue that is being made, while there is no issue yet. */
  readonly issueType?: string | undefined;
  /** Sends the people from now on, with their names; `null` takes the value away. */
  readonly onCommit: (picks: readonly Picked[] | Picked | null) => void;
}

/** A person who may be named: a button to choose them, or the word that they are and a button to take them out. */
function PersonRow({
  person,
  chosen,
  onPick,
  onTakeOut,
}: {
  readonly person: NamedPerson;
  readonly chosen: boolean;
  readonly onPick: (person: NamedPerson) => void;
  readonly onTakeOut: (id: string) => void;
}) {
  return (
    <PickerRow label={person.name}>
      {chosen ? (
        <>
          <PickerHint>{t("fields.pick-chosen")}</PickerHint>
          <Button
            aria-label={t("fields.pick-take-out-name", { name: person.name })}
            onPress={() => {
              onTakeOut(person.id);
            }}
          >
            {t("fields.pick-take-out")}
          </Button>
        </>
      ) : (
        <Button
          aria-label={t("fields.pick-choose-name", { name: person.name })}
          onPress={() => {
            onPick(person);
          }}
        >
          {t("fields.choose")}
        </Button>
      )}
    </PickerRow>
  );
}

/** The people who may be named, found by name; each row picks or takes out one person. */
function PeopleList({
  field,
  chosen,
  project,
  issue,
  issueType,
  onPick,
  onTakeOut,
}: {
  readonly field: OwnField;
  readonly chosen: readonly Picked[];
  readonly project: string;
  readonly issue: string | undefined;
  readonly issueType: string | undefined;
  readonly onPick: (person: NamedPerson) => void;
  readonly onTakeOut: (id: string) => void;
}) {
  const { words, frame } = usePickerFrame();
  const found = useAssignablePeople(project, words, { field: field.id, issue, issueType });
  const people = peopleOf(found.data?.pages);
  return (
    <SearchPicker
      legend={t("fields.choose-title", { name: field.name })}
      searchLabel={t("fields.pick-search")}
      listLabel={t("fields.pick-people-list")}
      {...frame}
      {...pickerStatus(found, people.length, {
        nothing: t("fields.pick-nothing"),
        more: t("fields.pick-more-people"),
      })}
    >
      {people.map((person) => (
        <PersonRow
          key={person.id}
          person={person}
          chosen={chosen.some((each) => each.id === person.id)}
          onPick={onPick}
          onTakeOut={onTakeOut}
        />
      ))}
    </SearchPicker>
  );
}

/** *Person*: one person of the project; *People*: several. Only people shown by name can be chosen. */
export function PeopleChoice({
  field,
  value,
  project,
  issue,
  issueType,
  onCommit,
}: PeopleChoiceProps) {
  const many = field.kind === "people";
  const chosen = picksOf(value);
  return (
    <ChoiceDialog field={field}>
      {(done) => (
        <PeopleList
          field={field}
          chosen={chosen}
          project={project}
          issue={issue}
          issueType={issueType}
          onPick={(person) => {
            if (many) {
              onCommit([...chosen, person]);
            } else {
              onCommit(person);
              done();
            }
          }}
          onTakeOut={(id) => {
            const left = chosen.filter((each) => each.id !== id);
            onCommit(left.length === 0 ? null : left);
          }}
        />
      )}
    </ChoiceDialog>
  );
}
