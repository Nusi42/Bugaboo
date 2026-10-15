import { useLayout, useUpright } from "@bugaboo/ui";

/** Where the open issue is shown, as the window allows (UX §3.2). */
export interface IssueTakesScreen {
  /** A sheet over the list: a tablet held upright has no room for a panel next to it. */
  readonly sheet: boolean;
  /** The issue takes the whole screen (a phone, or the sheet), so closing it leads back to the list. */
  readonly whole: boolean;
}

/** The one place that decides whether the open issue takes the whole screen. */
export function useIssueTakesScreen(): IssueTakesScreen {
  const layout = useLayout();
  const upright = useUpright();
  const sheet = layout === "tablet" && upright;
  return { sheet, whole: sheet || layout === "phone" };
}
