import type { SettingChoice } from "@bugaboo/api-client";

import { issueAddress, issuePanelAddress } from "../../shared/paths.ts";

/** How an issue is open: in the focus view or in the side panel (the choices of *Open issues in*). */
export type IssueShow = SettingChoice<"appearanceIssueView">;

/** The two ways, by name. */
export const issueShow = {
  focus: "focus",
  panel: "panel",
} as const satisfies Record<string, IssueShow>;

/** Where the title of an issue goes: the focus view, or the list of its project with the issue in the side panel. */
export function hrefOf(openIn: IssueShow, project: string, key: string): string {
  return openIn === issueShow.focus ? issueAddress(key) : issuePanelAddress(project, key);
}
