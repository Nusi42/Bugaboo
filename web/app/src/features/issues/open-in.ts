import { settingChoices, settingDefaults, settingKeys } from "@bugaboo/api-client";

import { useMySettings } from "../../shared/use-my-settings.ts";
import { oneOf, textOf } from "../../shared/setting-values.ts";
import type { IssueShow } from "./issue-show.ts";

/**
 * Where the person has chosen to open issues (*Open issues in*, UX §3.1, M2-F70): the focus view or
 * the side panel. The setting is read here and nowhere else; a value that is no longer a choice
 * opens the focus view.
 */
export function useOpenIssuesIn(): IssueShow {
  const settings = useMySettings();
  return oneOf(
    settingChoices.appearanceIssueView,
    textOf(settings, settingKeys.appearanceIssueView, settingDefaults.appearanceIssueView),
    settingDefaults.appearanceIssueView,
  );
}
