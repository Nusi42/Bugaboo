import { settingChoices, settingDefaults, settingKeys } from "@bugaboo/api-client";
import { useLayout } from "@bugaboo/ui";

import { useMySettings } from "../../shared/use-my-settings.ts";
import { oneOf, textOf } from "../../shared/setting-values.ts";
import { issueShow, type IssueShow } from "./issue-show.ts";

/**
 * Where the person has chosen to open issues (*Open issues in*, UX §3.1, M2-F70): the focus view or
 * the side panel. The setting is read here and nowhere else; a value that is no longer a choice
 * opens the focus view. A phone has no room for a panel next to the list: an issue always opens as
 * its full page there, with Back to the list (UX §3.2, maintainer decision).
 */
export function useOpenIssuesIn(): IssueShow {
  const settings = useMySettings();
  const phone = useLayout() === "phone";
  const chosen = oneOf(
    settingChoices.appearanceIssueView,
    textOf(settings, settingKeys.appearanceIssueView, settingDefaults.appearanceIssueView),
    settingDefaults.appearanceIssueView,
  );
  return phone ? issueShow.focus : chosen;
}
