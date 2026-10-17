import { settingKeys, type SettingKey } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import type { MySettings, SettingValue } from "../../../shared/my-settings.ts";
import type { OptimisticMutation } from "../../../shared/use-optimistic.ts";
import { numberIn } from "../../../shared/number.ts";

/** What the form changes in the settings of the organization. */
export type SettingEdits = OptimisticMutation<{
  readonly kind: "change";
  readonly key: SettingKey;
  readonly value: string | number | boolean;
}>;

/** What the person has typed into the form of the mail server. */
export interface Draft {
  readonly host: string;
  readonly port: string;
  readonly security: string;
  readonly username: string;
  readonly from: string;
  /** A new password; empty when the one that is stored stays. */
  readonly password: string;
}

/** One setting by its key. */
function settingOf(settings: MySettings, key: SettingKey) {
  return settings.settings.find((each) => each.key === key);
}

/** What a setting holds now, as the text of a box. */
function textOf(settings: MySettings, key: SettingKey): string {
  const value = settingOf(settings, key)?.value;
  return value === undefined ? "" : String(value);
}

/** The form as it starts: what the settings hold now, and no new password. */
export function draftOf(settings: MySettings): Draft {
  return {
    host: textOf(settings, settingKeys.mailHost),
    port: textOf(settings, settingKeys.mailPort),
    security: textOf(settings, settingKeys.mailSecurity),
    username: textOf(settings, settingKeys.mailUsername),
    from: textOf(settings, settingKeys.mailFrom),
    password: "",
  };
}

/** What saving does: the settings whose value differs, or the problem with the port. */
export interface Changes {
  readonly changes: readonly (readonly [SettingKey, SettingValue])[];
  /** Why nothing is saved: the port is not a number inside the bounds. */
  readonly problem?: string;
}

/**
 * The settings to save for a draft: those whose value differs from what the settings hold. A port
 * that is not a whole number between the bounds is not sent, because the server would only refuse
 * it; the server still decides everything else.
 */
export function changesOf(settings: MySettings, draft: Draft): Changes {
  const bounds = settingOf(settings, settingKeys.mailPort)?.bounds ?? undefined;
  const port = numberIn(draft.port, bounds);
  if (port === null) {
    return {
      changes: [],
      problem:
        bounds === undefined
          ? t("setting.ui.whole-number")
          : t("setting.ui.between", { least: bounds.least, most: bounds.most }),
    };
  }
  const wanted: readonly (readonly [SettingKey, SettingValue])[] = [
    [settingKeys.mailHost, draft.host.trim()],
    [settingKeys.mailPort, port],
    [settingKeys.mailSecurity, draft.security],
    [settingKeys.mailUsername, draft.username.trim()],
    [settingKeys.mailFrom, draft.from.trim()],
  ];
  return { changes: wanted.filter(([key, value]) => settingOf(settings, key)?.value !== value) };
}
