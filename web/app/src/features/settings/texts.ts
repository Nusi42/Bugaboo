import { isMessageKey, t } from "@bugaboo/messages";

import { type Setting } from "../../shared/my-settings.ts";

/**
 * The words for a key of the message catalog that the server named. A key this version of the
 * app does not have (the server may be newer) shows as the key itself, so nothing is empty.
 */
export function wordsOf(key: string): string {
  return isMessageKey(key) ? t(key) : key;
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
