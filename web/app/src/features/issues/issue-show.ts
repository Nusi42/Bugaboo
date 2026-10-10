import type { SettingChoice } from "@bugaboo/api-client";

/** How an issue is open: in the focus view or in the side panel (the choices of *Open issues in*). */
export type IssueShow = SettingChoice<"appearanceIssueView">;

/** The two ways, by name. */
export const issueShow = {
  focus: "focus",
  panel: "panel",
} as const satisfies Record<string, IssueShow>;
