import { settingDefaults, settingKeys } from "@bugaboo/api-client";

import { mySettingsOptions, type MySettings } from "./my-settings.ts";
import { queryClient } from "./query-client.ts";
import { textOf } from "./setting-values.ts";

/**
 * Whether an address is a page of this app: one slash at the start, so never another website
 * (`//evil.test`, `https://…`, `/\evil.test`). The server checks the same when it stores the
 * address; the browser checks again, because it is the one that navigates.
 */
function isPageOfTheApp(address: string): boolean {
  return address.startsWith("/") && !address.startsWith("//") && !address.startsWith("/\\");
}

/** The address of the page that opens after signing in; the home page for any other address. */
export function startPageOf(settings?: MySettings): string {
  const address = textOf(settings, settingKeys.startPage, settingDefaults.startPage);
  return isPageOfTheApp(address) ? address : settingDefaults.startPage;
}

/** The page that opens after signing in: the person's start page, or the home page. */
export async function loadStartPage(): Promise<string> {
  try {
    return startPageOf(await queryClient.query(mySettingsOptions()));
  } catch {
    return settingDefaults.startPage;
  }
}
