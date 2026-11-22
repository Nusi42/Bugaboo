import { t } from "@bugaboo/messages";
import { PickerOption, SearchPicker } from "@bugaboo/ui";

import { choicesOf, type Choice } from "../../shared/choices.ts";
import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { useExportPeople } from "./data-exports.ts";

/** A person to choose, by name. */
export type Picked = Choice;

/**
 * Whose data: the people the holder of *Download someone's data* may name, as *Who sees people* lets them, found by the
 * words typed (one person picker, M5-F135). The permission brings this list; *Manage people* is not needed.
 */
export function PersonPicker({
  picked,
  onPick,
}: {
  readonly picked: Picked | undefined;
  readonly onPick: (person: Picked) => void;
}) {
  const { words, group, frame } = usePickerFrame();
  const found = useExportPeople(words);
  const people = choicesOf(found.data?.pages);
  return (
    <SearchPicker
      legend={t("data-exports.pick")}
      searchLabel={t("data-exports.pick-search")}
      listLabel={t("data-exports.pick-list")}
      {...frame}
      {...pickerStatus(found, people.length, {
        nothing: t("data-exports.pick-nothing"),
        more: t("data-exports.pick-more"),
      })}
    >
      {people.map((person) => (
        <PickerOption
          key={person.id}
          group={group}
          checked={picked?.id === person.id}
          label={person.name}
          onPick={() => {
            onPick({ id: person.id, name: person.name });
          }}
        />
      ))}
    </SearchPicker>
  );
}
