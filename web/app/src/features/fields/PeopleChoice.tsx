import { t } from "@bugaboo/messages";
import { Button, PickerHint, PickerRow, SearchPicker } from "@bugaboo/ui";

import type { NamedPerson } from "../../shared/person.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { peopleOf, useAssignablePeople } from "../issues/index.ts";
import { ChoiceDialog } from "./ChoiceDialog.tsx";
import type { OwnField } from "./fields.ts";
import { referenceIds } from "./reference.ts";

export interface PeopleChoiceProps {
  readonly field: OwnField;
  /** The value now, as the server showed it. */
  readonly value: unknown;
  /** The project of the item (its short name): the people of the project are offered. */
  readonly project: string;
  /** The issue (its ID), when the item is one: the people who may see it are offered. */
  readonly issue?: string | undefined;
  /** Sends the IDs of the people from now on; `null` takes the value away. */
  readonly onCommit: (ids: readonly string[] | string | null) => void;
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
  readonly onPick: (id: string) => void;
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
            onPick(person.id);
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
  onPick,
  onTakeOut,
}: {
  readonly field: OwnField;
  readonly chosen: readonly string[];
  readonly project: string;
  readonly issue: string | undefined;
  readonly onPick: (id: string) => void;
  readonly onTakeOut: (id: string) => void;
}) {
  const { words, frame } = usePickerFrame();
  const found = useAssignablePeople(project, words, { field: field.id, issue });
  const people = peopleOf(found.data?.pages);
  return (
    <SearchPicker
      legend={t("fields.choose-title", { name: field.name })}
      searchLabel={t("fields.pick-search")}
      listLabel={t("fields.pick-people-list")}
      {...frame}
      {...pickerStatus(found, people.length, {
        nothing: t("fields.pick-nothing"),
        more: t("fields.pick-more"),
      })}
    >
      {people.map((person) => (
        <PersonRow
          key={person.id}
          person={person}
          chosen={chosen.includes(person.id)}
          onPick={onPick}
          onTakeOut={onTakeOut}
        />
      ))}
    </SearchPicker>
  );
}

/** *Person*: one person of the project; *People*: several. Only people shown by name can be chosen. */
export function PeopleChoice({ field, value, project, issue, onCommit }: PeopleChoiceProps) {
  const many = field.kind === "people";
  const chosen = referenceIds(value);
  return (
    <ChoiceDialog field={field}>
      {(done) => (
        <PeopleList
          field={field}
          chosen={chosen}
          project={project}
          issue={issue}
          onPick={(id) => {
            if (many) {
              onCommit([...chosen, id]);
            } else {
              onCommit(id);
              done();
            }
          }}
          onTakeOut={(id) => {
            const left = chosen.filter((each) => each !== id);
            onCommit(left.length === 0 ? null : left);
          }}
        />
      )}
    </ChoiceDialog>
  );
}
