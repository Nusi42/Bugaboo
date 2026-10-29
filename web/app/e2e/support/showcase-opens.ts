import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { check } from "./access.ts";
import { addressWith, sendPassword } from "./flows.ts";
import { layoutOf } from "./layout.ts";
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

/** A screen that is a page of issues at an address: the picture is taken when the list has its first rows, not before the data came. */
export function atList(address: string): Screen["open"] {
  return async ({ page }) => {
    await page.goto(address);
    await expectPage(page);
    await expect(page.locator("[data-part='list-row']").first()).toBeVisible();
  };
}

/** The list with an issue open in the side panel: the picture waits for the count line of the list; a phone has no list beside the issue. */
export function atPanel(address: string): Screen["open"] {
  return async ({ page }) => {
    await page.goto(address);
    await expectPage(page);
    if (layoutOf(page) !== "phone") {
      await expect(page.getByText(/ shown$/u).first()).toBeVisible();
    }
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
  // Where the tree is a sheet (a phone) it is the sheet More, which the picture shows from its top.
  await page
    .getByRole("dialog", { name: t("shell.more") })
    .evaluateAll((nodes: readonly HTMLElement[]) => {
      for (const node of nodes) {
        node.scrollTop = 0;
      }
    });
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

/** The address the invitation of the picture is for: always the same, so the picture is. */
const NEW_COLLEAGUE = "new.colleague@bugaboo.example";

/** Takes back every open invitation for this address, one after the other, until none is left. */
async function takeBackInvitationsOf(page: Page, email: string): Promise<void> {
  // Only the rows of the list of open invitations (a list inside a card inside a list would count the card too),
  // counted once the list has been read.
  const open = page.getByRole("list", { name: t("invitations.open") });
  await expect(open.or(page.getByText(t("invitations.none")))).toBeVisible();
  const rows = open.getByRole("listitem").filter({ hasText: email });
  const before = await rows.count();
  if (before === 0) {
    return;
  }
  await rows
    .first()
    .getByRole("button", { name: new RegExp(`^${t("invitations.take-back")}`, "u") })
    .click();
  // The question that follows is answered with the same words.
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("invitations.take-back"), exact: true })
    .click();
  await expect(rows).toHaveCount(before - 1);
  await takeBackInvitationsOf(page, email);
}

/** The people of the organization, after the invitation of an earlier theme was taken back: every theme shows the same. */
export async function openPeople({ page }: Needs): Promise<void> {
  await page.goto(paths.people);
  await expectPage(page);
  await takeBackInvitationsOf(page, NEW_COLLEAGUE);
}

/** Makes an invitation for somebody new, so that the link that shows once is in the picture. */
export async function openInvitation({ page }: Needs): Promise<void> {
  await page.goto(paths.people);
  await expectPage(page);
  const email = NEW_COLLEAGUE;
  // An invitation of an earlier theme is taken back first, so every theme shows the same list (M5 exit, O11).
  await takeBackInvitationsOf(page, email);
  await page.getByRole("textbox", { name: t("field.email"), exact: true }).fill(email);
  await page.getByRole("button", { name: t("invitations.create"), exact: true }).click();
  await expect(page.getByText(t("invitations.created", { email }))).toBeVisible();
}
