import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { check } from "./access.ts";
import { addressWith, sendPassword } from "./flows.ts";
import { openMove, tree } from "./projects.ts";
import { sampleWithCodeApp } from "./sample.ts";
import { PROJECT, showcaseNames, SUB_PROJECT, type ShowcaseIds } from "./showcase-sample.ts";
import { type Needs, type Screen } from "./showcase.ts";
import { addressInProject, paths, projectAddress } from "../../src/shared/paths.ts";

/** The page of a screen has its heading. */
export async function expectPage(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1 }).first()).toBeVisible();
}

/** A screen that is a page at an address. */
export function at(address: string | ((ids: ShowcaseIds) => string)): Screen["open"] {
  return async ({ page, ids }) => {
    await page.goto(typeof address === "string" ? address : address(ids));
    await expectPage(page);
  };
}

/** Signs in as a person with a code app: the second step of signing in then asks for its code. */
export async function secondStep({ page }: Needs): Promise<void> {
  await sendPassword(page, sampleWithCodeApp().person);
  await expect(page).toHaveURL(addressWith(paths.secondStep));
  await expect(page.getByRole("heading", { level: 1, name: t("second-step.title") })).toBeVisible();
}

/** Opens the tree of projects with the sub-projects of the project shown. */
export async function openTree({ page }: Needs): Promise<void> {
  await page.goto(projectAddress(PROJECT));
  await expectPage(page);
  const region = await tree(page);
  const sub = region.getByRole("link", { name: /^iOS/u });
  if (!(await sub.isVisible())) {
    await region.getByRole("button", { name: /Mobile app/u }).click();
  }
  await expect(sub).toBeVisible();
}

/** Opens quick search with a word typed, and waits for what it found. */
export async function openQuickSearch({ page }: Needs): Promise<void> {
  await page.goto(paths.home);
  await expectPage(page);
  await page.keyboard.press("Control+k");
  await page.getByRole("searchbox", { name: t("search.label"), exact: true }).fill("web");
  await expect(page.getByRole("option").first()).toBeVisible();
}

/** Checks what a person can do in the whole organization, and waits for the answer. */
export async function openCheckAccess({ page }: Needs): Promise<void> {
  await page.goto(paths.checkAccess);
  await expectPage(page);
  await check(page, null, { name: showcaseNames.person });
  await expect(page.getByRole("heading", { level: 2 }).first()).toBeVisible();
}

/** Moves a sub-project to the top level in the dialog, up to the preview of what changes. */
export async function openMovePreview({ page }: Needs): Promise<void> {
  await openMove(page, addressInProject(paths.projectSettings, SUB_PROJECT));
}

/** A screen of the page frames, once what it shows is there. */
export function inFrames(search: string, ready: (page: Page) => Promise<void>): Screen["open"] {
  return async ({ page }) => {
    await page.goto(`${paths.frames}${search}`);
    await ready(page);
  };
}

/** Makes an invitation for somebody new, so that the link that shows once is in the picture. */
export async function openInvitation({ page }: Needs): Promise<void> {
  await page.goto(paths.people);
  await expectPage(page);
  const email = `new.colleague.${String(Date.now())}@bugaboo.example`;
  await page.getByRole("textbox", { name: t("field.email"), exact: true }).fill(email);
  await page.getByRole("button", { name: t("invitations.create"), exact: true }).click();
  await expect(page.getByText(t("invitations.created", { email }))).toBeVisible();
}
