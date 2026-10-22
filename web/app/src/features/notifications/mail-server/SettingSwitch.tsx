import { type SettingKey } from "@bugaboo/api-client";
import { Switch, partOf, part } from "@bugaboo/ui";

import { settingOf, type MySettings } from "../../../shared/my-settings.ts";
import { wordsOf } from "../../settings/index.ts";
import { type SettingEdits } from "./connection-draft.ts";

/** A switch for a yes or no setting of the organization, with its explanation under it. */
export function SettingSwitch({
  settings,
  edits,
  settingKey,
}: {
  readonly settings: MySettings;
  readonly edits: SettingEdits;
  readonly settingKey: SettingKey;
}) {
  const found = settingOf(settings, settingKey);
  if (found === undefined) {
    return null;
  }
  return (
    <div {...partOf(part.column)}>
      <Switch
        label={wordsOf(found.name_key)}
        isSelected={found.value === true}
        onChange={(next) => {
          edits.change({ kind: "change", key: settingKey, value: next });
        }}
      />
      <p {...partOf(part.muted)}>{wordsOf(found.help_key)}</p>
    </div>
  );
}
