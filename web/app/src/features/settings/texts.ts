import { isMessageKey, t } from "@bugaboo/messages";

import { type Setting } from "../../shared/my-settings.ts";
import type { SettingActions } from "./SettingRow.tsx";

/**
 * The words for a key of the message catalog that the server named. A key this version of the
 * app does not have (the server may be newer) shows as the key itself, so nothing is empty.
 */
export function wordsOf(key: string): string {
  return isMessageKey(key) ? t(key) : key;
}

/**
 * Whether a setting has no generic control: the keys of shortcuts and the start page have pages of
 * their own, and the sizes of the windows are kept by the windows when a person resizes one.
 */
export function hasNoGenericControl(setting: Setting): boolean {
  return (
    setting.format === "shortcut-keys" ||
    setting.format === "page-address" ||
    setting.format === "window-sizes"
  );
}

/** Where the value comes from, in the words of the glossary: the standard, the organization, a project, you. */
export function sourceInWords(setting: Setting): string {
  const { kind, project } = setting.source;
  if (kind === "default") {
    return t("setting.ui.source-default");
  }
  if (kind === "organization") {
    return t("setting.ui.source-organization");
  }
  if (kind === "person") {
    return t("setting.ui.source-person");
  }
  return project === undefined || project === null
    ? t("setting.ui.source-hidden-project")
    : t("setting.ui.source-project", { project: project.short_name });
}

/** The kinds of control a setting is drawn with. */
export type ControlKind = "switch" | "choices" | "list" | "time-zone" | "none" | "typed";

/**
 * The control that fits what a setting holds: the one place that says it. A switch for yes or no, a few
 * buttons (up to four choices) or a list for a fixed list of choices, a list of time zones, nothing for a
 * setting that has a page of its own, and else a box that is typed in, with its own Save button and its hint.
 */
export function controlKindOf(setting: Setting): ControlKind {
  const value = setting.value;
  if (setting.kind === "yes-no" && typeof value === "boolean") {
    return "switch";
  }
  if (setting.format === "choice" && typeof value === "string") {
    return setting.choices.length <= 4 ? "choices" : "list";
  }
  if (setting.format === "time-zone" && typeof value === "string") {
    return "time-zone";
  }
  return hasNoGenericControl(setting) ? "none" : "typed";
}

/** Whether an administrator locked the setting above this level: it is shown read only and cannot be changed here. */
export function isLockedBelow(setting: Setting, actions: Pick<SettingActions, "onLock">): boolean {
  return setting.locked && actions.onLock === undefined;
}
