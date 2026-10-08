import type { ReadonlyDeep, SettingKey } from "@bugaboo/api-client";

import type { MySettings, Setting, SettingValue } from "./my-settings.ts";

/** One change of a person's own settings: a new value, or `null` to take it away. */
export interface SettingChange {
  readonly key: SettingKey;
  readonly value: SettingValue | null;
}

/** The settings after a change, before the server has answered: the value, as the person's own. */
export function withChange(before: MySettings, change: SettingChange): MySettings {
  return {
    settings: before.settings.map((setting): ReadonlyDeep<Setting> => {
      if (setting.key !== change.key) {
        return setting;
      }
      if (change.value === null) {
        // The value from above is not known here; the answer of the server brings it.
        return { ...setting, value: setting.default, own: false, source: { kind: "default" } };
      }
      return { ...setting, value: change.value, own: true, source: { kind: "person" } };
    }),
  };
}
