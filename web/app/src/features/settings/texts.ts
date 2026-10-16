import { isMessageKey, t } from "@bugaboo/messages";

import { type Setting } from "../../shared/my-settings.ts";

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
    setting.format === "shortcut_keys" ||
    setting.format === "page_address" ||
    setting.format === "window_sizes"
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
