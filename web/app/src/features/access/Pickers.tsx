import { t } from "@bugaboo/messages";
import { PickerOption, pickerLook, SearchPicker } from "@bugaboo/ui";

import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { peopleOf, placesOf, usePeopleToCheck, useProjectsToChoose } from "./use-check.ts";

export interface PersonPickerProps {
  /** The project the person is checked in (an ID); the whole organization without one. */
  readonly project: string | undefined;
  /** The person picked (an ID), if there is one. */
  readonly value: string | undefined;
  readonly onChange: (person: string) => void;
}

/**
 * The person to check: the people whose names the person who checks may see, found by words. Only
 * names are listed; somebody who may not see names is offered nobody.
 */
export function PersonPicker({ project, value, onChange }: PersonPickerProps) {
  const { words, group, frame } = usePickerFrame();
  const found = usePeopleToCheck(project, words);
  const people = peopleOf(found.data?.pages);
  return (
    <SearchPicker
      look={pickerLook.boxed}
      legend={t("check-access.person-list")}
      searchLabel={t("check-access.person-search")}
      listLabel={t("check-access.person-list")}
      {...frame}
      {...pickerStatus(found, people.length, {
        nothing: t("check-access.person-nothing"),
        more: t("check-access.person-more"),
      })}
    >
      {people.map((person) => (
        <PickerOption
          key={person.id}
          group={group}
          checked={value === person.id}
          label={person.name}
          onPick={() => {
            onChange(person.id);
          }}
        />
      ))}
    </SearchPicker>
  );
}

export interface PlacePickerProps {
  /** The project picked (an ID); `null` for the whole organization. */
  readonly value: string | null;
  readonly onChange: (project: string | null) => void;
}

/** Where to look: the whole organization, or one of the projects the person sees, found by words. */
export function PlacePicker({ value, onChange }: PlacePickerProps) {
  const { words, group, frame } = usePickerFrame();
  const found = useProjectsToChoose(words);
  const places = placesOf(found.data?.pages);
  return (
    <SearchPicker
      look={pickerLook.boxed}
      legend={t("check-access.place-legend")}
      searchLabel={t("check-access.place-search")}
      listLabel={t("check-access.place-list")}
      {...frame}
      {...pickerStatus(found, places.length, {
        nothing: t("check-access.place-nothing"),
        more: t("check-access.place-more"),
      })}
    >
      <PickerOption
        group={group}
        checked={value === null}
        label={t("check-access.place-everywhere")}
        onPick={() => {
          onChange(null);
        }}
      />
      {places.map((place) => (
        <PickerOption
          key={place.id}
          group={group}
          checked={value === place.id}
          label={place.name}
          hint={place.shortName}
          onPick={() => {
            onChange(place.id);
          }}
        />
      ))}
    </SearchPicker>
  );
}
