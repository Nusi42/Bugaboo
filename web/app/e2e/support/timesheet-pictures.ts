import { settingKeys } from "@bugaboo/api-client";
import { type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { inFourChosenThemes } from "./four-themes.ts";
import { setOrganizationSwitch } from "./organization.ts";
import { type Instance } from "./server.ts";

/** Where the pictures of the timesheet go, below the folder the run was given. */
const MILESTONE = "M6/timesheet";

/** Takes the screen `open` shows in the four themes on this window, as the maintainer reviews it. */
export function pictures(page: Page, name: string, open: () => Promise<void>): Promise<void> {
  return inFourChosenThemes(page, name, MILESTONE, paths.appearance, open);
}

/** Turns *Totals only* on, as an administrator does on the settings page. */
export function switchTotalsOnly(server: Instance): Promise<void> {
  return setOrganizationSwitch(server, settingKeys.workingHoursTotalsOnly, true);
}
