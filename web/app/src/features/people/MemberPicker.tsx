import { t } from "@bugaboo/messages";
import { Button, PickerHint, PickerRow, SearchPicker } from "@bugaboo/ui";

import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { recentFirst } from "../../shared/recent-choices.ts";
import { choicesOf, useMemberChoices, type GroupLine } from "./groups.ts";
import type { MemberChanges } from "./use-member-changes.ts";

export interface MemberPickerProps {
  readonly group: GroupLine;
  readonly changes: MemberChanges;
}

/**
 * Putting people into the group (UX §6): a search box, and the people it finds by name with a button
 * each; those who are in already say so, and the people picked last come first.
 */
export function MemberPicker({ group, changes }: MemberPickerProps) {
  const { words, frame } = usePickerFrame();
  const found = useMemberChoices(group.id, words);
  const choices = choicesOf(found.data?.pages);
  const shown = words === "" ? recentFirst("person", choices) : choices;
  return (
    <SearchPicker
      legend={t("groups.add-title")}
      searchLabel={t("groups.add-search")}
      listLabel={t("groups.add-list")}
      {...frame}
      {...pickerStatus(found, shown.length, {
        nothing: t("groups.add-nothing"),
        more: t("groups.add-more"),
      })}
    >
      {shown.map((person) => (
        <PickerRow key={person.id} label={person.name}>
          {person.alreadyIn ? (
            <PickerHint>{t("groups.in-already")}</PickerHint>
          ) : (
            <Button
              aria-label={t("groups.add-name", { name: person.name })}
              onPress={() => {
                changes.add(person);
              }}
            >
              {t("groups.add")}
            </Button>
          )}
        </PickerRow>
      ))}
    </SearchPicker>
  );
}
