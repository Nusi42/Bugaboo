import { type Setting, type SettingValue } from "../../shared/my-settings.ts";

/** What a row can do with its setting. */
export interface SettingActions {
  /** A new value, to be shown at once and sent. */
  readonly onChange: (key: string, value: SettingValue) => void;
  /** *Reset to default*: takes the value of this level away. */
  readonly onReset: (key: string) => void;
  /** Locks or unlocks the organization's value; only given on the organization's own level. */
  readonly onLock?: (key: string, locked: boolean) => void;
}

/** Whether an administrator locked the setting above this level: it is shown read only and cannot be changed here. */
export function isLockedBelow(setting: Setting, actions: SettingActions): boolean {
  return setting.locked && actions.onLock === undefined;
}
