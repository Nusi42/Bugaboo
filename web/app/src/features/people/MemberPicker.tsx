import { t } from "@bugaboo/messages";
import { Button, PickerOption, SearchPicker, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { recentFirst } from "../../shared/recent-choices.ts";
import { choicesOf, useMemberChoices, type Choice, type GroupLine } from "./groups.ts";
import type { MemberChanges } from "./use-member-changes.ts";

export interface MemberPickerProps {
  readonly group: GroupLine;
  readonly changes: MemberChanges;
}

/**
 * Putting people into the group (UX §6), the way a person is picked to give a role to: a search box, the people it finds
 * by name as radio buttons, the ones picked last first, and *Put in* under them for the one picked. Those who are in
 * already say so, and cannot be put in again (M5-F135: one person picker).
 */
export function MemberPicker({ group, changes }: MemberPickerProps) {
  const { words, group: radios, frame } = usePickerFrame();
  const [picked, setPicked] = useState<Choice>();
  const found = useMemberChoices(group.id, words);
  const choices = choicesOf(found.data?.pages);
  const shown = words === "" ? recentFirst("person", choices) : choices;
  return (
    <div {...partOf(part.stack)}>
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
          <PickerOption
            key={person.id}
            group={radios}
            checked={picked?.id === person.id}
            label={person.name}
            {...(person.alreadyIn ? { hint: t("groups.in-already") } : {})}
            onPick={() => {
              setPicked(person);
            }}
          />
        ))}
      </SearchPicker>
      <div {...partOf(part.row)}>
        <Button
          isDisabled={picked === undefined || picked.alreadyIn}
          {...(picked === undefined
            ? {}
            : { "aria-label": t("groups.add-name", { name: picked.name }) })}
          onPress={() => {
            if (picked !== undefined) {
              changes.add(picked);
              setPicked(undefined);
            }
          }}
        >
          {t("groups.add")}
        </Button>
      </div>
    </div>
  );
}
