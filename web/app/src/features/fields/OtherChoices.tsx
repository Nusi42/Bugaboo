import { t } from "@bugaboo/messages";
import { Button, PickerRow, SearchPicker } from "@bugaboo/ui";

import { pickerStatus, usePickerFrame } from "../../shared/picker-status.ts";
import { groupLinesOf, useGroups } from "../people/index.ts";
import { ChoiceDialog } from "./ChoiceDialog.tsx";
import type { OwnField } from "./fields.ts";
import type { Picked } from "../../shared/reference.ts";

export interface OtherChoiceProps {
  readonly field: OwnField;
  /** Sends the thing chosen: its ID and its name. */
  readonly onCommit: (pick: Picked) => void;
}

/** *Group*: the groups the person may list, found by name. */
export function GroupChoice({ field, onCommit }: OtherChoiceProps) {
  const { frame } = usePickerFrame();
  const found = useGroups();
  const groups = groupLinesOf(found.data?.pages);
  return (
    <ChoiceDialog field={field}>
      {(done) => (
        <SearchPicker
          legend={t("fields.choose-title", { name: field.name })}
          searchLabel={t("fields.pick-search")}
          listLabel={t("fields.pick-groups-list")}
          {...frame}
          {...pickerStatus(found, groups.length, {
            nothing: t("fields.pick-no-groups"),
            more: t("fields.pick-more-groups"),
          })}
        >
          {groups
            .filter((group) => group.name.toLowerCase().includes(frame.query.trim().toLowerCase()))
            .map((group) => (
              <PickerRow key={group.id} label={group.name}>
                <Button
                  aria-label={t("fields.pick-choose-name", { name: group.name })}
                  onPress={() => {
                    onCommit({ id: group.id, name: group.name });
                    done();
                  }}
                >
                  {t("fields.choose")}
                </Button>
              </PickerRow>
            ))}
        </SearchPicker>
      )}
    </ChoiceDialog>
  );
}
