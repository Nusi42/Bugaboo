import { t } from "@bugaboo/messages";
import { PickerOption, SearchPicker } from "@bugaboo/ui";

import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { recentFirst } from "../../shared/recent-choices.ts";
import { choicesOf, useChoices, type Choice } from "./assignments.ts";
import { type SubjectKind } from "../../shared/person.ts";

export interface ChoicePickerProps {
  /** Whom to pick: a person, or a group. */
  readonly kind: SubjectKind;
  /** The project the row is for (an ID), or `null` for the whole organization. */
  readonly project: string | null;
  /** The ID of the one picked, if any. */
  readonly value: string | null;
  readonly onPick: (choice: Choice) => void;
}

/**
 * Whom to give a role to (UX §6): a search box and the people or groups it finds by name, as radio
 * buttons; the ones picked last come first. Names only: the server never offers an e-mail address.
 */
export function ChoicePicker({ kind, project, value, onPick }: ChoicePickerProps) {
  const { words, group, frame } = usePickerFrame();
  const found = useChoices(kind, project, words);
  const all = choicesOf(found.data?.pages);
  const shown = words === "" ? recentFirst(kind, all) : all;
  const people = kind === "person";
  return (
    <SearchPicker
      legend={t(people ? "assignments.pick-person" : "assignments.pick-group")}
      searchLabel={t(people ? "assignments.search-people" : "assignments.search-groups")}
      listLabel={t("assignments.pick-list")}
      {...frame}
      {...pickerStatus(found, shown.length, {
        nothing: t("assignments.pick-nothing"),
        more: t("assignments.pick-more"),
      })}
    >
      {shown.map((choice) => (
        <PickerOption
          key={choice.id}
          group={group}
          checked={value === choice.id}
          label={choice.name}
          onPick={() => {
            onPick(choice);
          }}
        />
      ))}
    </SearchPicker>
  );
}
