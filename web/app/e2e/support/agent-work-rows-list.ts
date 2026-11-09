import { expect, type Page } from "@playwright/test";

import { projectIssuesAddress } from "../../src/shared/paths.ts";
import { ada } from "./accounts.ts";
import { rowsOfStatuses } from "./agent-work-rows.ts";
import { signIn } from "./flows.ts";
import { tickIssue } from "./list-look.ts";
import type { Instance } from "./server.ts";

/** Signs in, opens the list of the project with the rows of the three status groups and ticks the middle one; the first key. */
export async function openRowsList(page: Page, server: Instance): Promise<string> {
  const [first, second, third] = await rowsOfStatuses(server);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByText(third ?? "", { exact: true })).toBeVisible();
  await tickIssue(page, second ?? "");
  return first ?? "";
}
