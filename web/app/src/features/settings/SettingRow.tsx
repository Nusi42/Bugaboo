import { t } from "@bugaboo/messages";
import { Button, Switch } from "@bugaboo/ui";
import { type ReactNode } from "react";

import { type Setting, type SettingValue } from "../../shared/my-settings.ts";
import { SettingControl } from "./SettingControl.tsx";
import styles from "./Settings.module.css";
import { sourceInWords, wordsOf } from "./texts.ts";

/** What a row can do with its setting. */
export interface SettingActions {
  /** A new value, to be shown at once and sent. */
  readonly onChange: (key: string, value: SettingValue) => void;
  /** *Reset to default*: takes the value of this level away. */
  readonly onReset: (key: string) => void;
  /** Locks or unlocks the organization's value; only given on the organization's own level. */
  readonly onLock?: (key: string, locked: boolean) => void;
}

/** A control a page draws for one setting instead of the generic one (a theme with its sample). */
export type ControlRenderer = (
  setting: Setting,
  onChange: (value: SettingValue) => void,
) => ReactNode;

export interface SettingRowProps {
  readonly setting: Setting;
  readonly actions: SettingActions;
  /** The page's own control for this setting, if it has one. */
  readonly control?: ControlRenderer | undefined;
}

/** The value in words, for a setting that cannot be changed here. */
function valueInWords(setting: Setting): string {
  const value = setting.value;
  if (typeof value === "boolean") {
    return value ? t("common.on") : t("common.off");
  }
  const choice = setting.choices.find((one) => one.value === value);
  return choice === undefined ? String(value) : wordsOf(choice.label_key);
}

/** A setting an administrator locked: read only, with the reason (principle 10). */
function Locked({ setting }: { readonly setting: Setting }) {
  return (
    <p className={styles.locked}>
      <span className={styles.lockedTitle}>{wordsOf(setting.name_key)}</span>
      <span className={styles.lockedValue}>{valueInWords(setting)}</span>
      <strong>{t("setting.ui.locked")}</strong>
      <span className={styles.help}>{t("setting.ui.locked-why")}</span>
    </p>
  );
}

/**
 * One setting: its control with its name, the one-line explanation under it, where the value
 * comes from, *Reset to default* when this level has a value of its own, and, for an
 * administrator on the organization's level, the lock. A setting the organization locked is
 * shown read only and says so.
 */
export function SettingRow({ setting, actions, control }: SettingRowProps) {
  const lockedBelow = setting.locked && actions.onLock === undefined;
  const change = (value: SettingValue): void => {
    actions.onChange(setting.key, value);
  };
  return (
    <li className={styles.row}>
      {lockedBelow ? (
        <Locked setting={setting} />
      ) : (
        (control?.(setting, change) ?? <SettingControl setting={setting} onChange={change} />)
      )}
      <p className={styles.help}>{wordsOf(setting.help_key)}</p>
      <p className={styles.source}>{sourceInWords(setting)}</p>
      <div className={styles.actions}>
        {setting.own && !lockedBelow ? (
          <Button
            quiet
            aria-label={t("setting.ui.reset-name", { name: wordsOf(setting.name_key) })}
            onPress={() => {
              actions.onReset(setting.key);
            }}
          >
            {t("setting.ui.reset")}
          </Button>
        ) : null}
        {actions.onLock !== undefined && setting.lockable ? (
          <div>
            <Switch
              label={t("setting.ui.lock")}
              isSelected={setting.locked}
              onChange={(locked) => {
                actions.onLock?.(setting.key, locked);
              }}
            />
            <p className={styles.help}>{t("setting.ui.lock-help")}</p>
          </div>
        ) : null}
      </div>
    </li>
  );
}
