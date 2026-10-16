import { settingDefaults, settingKeys } from "@bugaboo/api-client";
import { parseWindowSizes, type WindowSize, type WindowSizes } from "@bugaboo/ui";

import type { MySettings } from "./my-settings.ts";
import type { SettingChange } from "./setting-change.ts";
import { textOf } from "./setting-values.ts";

/** The sizes the person gave the windows of their forms, by the name of the form. */
export function sizesOf(settings?: MySettings): WindowSizes {
  return parseWindowSizes(textOf(settings, settingKeys.windowSizes, settingDefaults.windowSizes));
}

/** The size the person gave the window of one form, or nothing when it has its usual size. */
export function sizeOfForm(settings: MySettings | undefined, form: string): WindowSize | undefined {
  return sizesOf(settings)[form];
}

/**
 * The change of the setting when the window of `form` ended up `size` big: the other forms keep
 * theirs. The server keeps whole pixels, so the size is rounded.
 */
export function changeForSize(
  settings: MySettings | undefined,
  form: string,
  size: WindowSize,
): SettingChange {
  const rounded = { width: Math.round(size.width), height: Math.round(size.height) };
  return {
    key: settingKeys.windowSizes,
    value: JSON.stringify({ ...sizesOf(settings), [form]: rounded }),
  };
}
