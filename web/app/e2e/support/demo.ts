import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { addressInProject, paths, projectAddress } from "../../src/shared/paths.ts";
import { chooseTheme, expectGoodInEveryLook, themes } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { inOrder } from "./sequence.ts";
import { sampleProjectName, sampleProjects } from "./sample.ts";
import { tree } from "./projects.ts";
import { shootWhole, shootWholeAs } from "./screenshots.ts";

// What the exit demo of M2 and its pictures share (m2-exit-demo.spec.ts).

/** The folder of the demo pictures inside the folder of the screenshots. */
const DEMO = "M2/demo";

/** What the pictures of a step are called: the step, the size of the screen, then the look. */
function picture(step: string, page: Page): string {
  return `${step}-${layoutOf(page) === "computer" ? "computer" : "phone"}`;
}

/** The screen is good in the four themes (it has no accessibility problem) and is in the pictures. */
export async function good(page: Page, step: string): Promise<void> {
  // The pictures first: the check puts every theme on the page, and leaves the last one there.
  await shootWhole(page, picture(step, page), DEMO);
  await expectGoodInEveryLook(page);
}

/** The folder of the pictures of the exit demo of M3. */
const DEMO_M3 = "M3/demo";

/**
 * The screen is good in each of the four themes and is in the pictures of the exit demo of M3, one
 * for each theme (`<step>-<phone|computer>-<theme>.png`): the screen is checked in each theme, and
 * then the pictures are taken when a folder is wanted.
 */
export async function goodInFourThemes(page: Page, step: string, folder = DEMO_M3): Promise<void> {
  // The checks first, in the window as it is: the window grown for a picture leaves the bars at
  // its edges in a state that no person ever sees.
  await expectGoodInEveryLook(page);
  await inOrder(
    themes.map((theme) => async () => {
      await chooseTheme(page, theme);
      await shootWholeAs(page, picture(step, page), folder, theme.id);
    }),
  );
}

/** Opens a page and waits for its heading. */
export async function open(page: Page, address: string, heading: string): Promise<void> {
  await page.goto(address);
  await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
}

/** A sentence of the answer of *Check access* or of a preview (the first place it is written). */
export function sentence(page: Page, text: string): Locator {
  return page.getByText(text).first();
}

/** The tree of projects shows the project and, as the person may see, no other of the sample. */
export async function seesOnlyTheTreeOf(page: Page, shortName: string): Promise<void> {
  const projects = await tree(page);
  const own = sampleProjectName(shortName);
  await expect(projects.getByRole("link", { name: new RegExp(`^${own}`, "u") })).toBeVisible();
  const others = sampleProjects.filter(
    (project) => project.inside === null && project.name !== own,
  );
  await inOrder(
    others.map((project) => async () => {
      await expect(projects.getByRole("link", { name: project.name, exact: true })).toHaveCount(0);
    }),
  );
}

/** The pages of the administrators, and a project the person has no part in, do not exist for them. */
export async function findsNoneOf(page: Page, own: string, other: string): Promise<void> {
  await inOrder(
    [
      paths.people,
      paths.roles,
      addressInProject(paths.projectFields, own),
      projectAddress(other),
    ].map((address) => async () => {
      await page.goto(address);
      await expect(
        page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
      ).toBeVisible();
    }),
  );
}

/** Closes the sheet *More* opened on a phone, so the page behind it can be used. */
export async function closeTheSheet(page: Page): Promise<void> {
  await (layoutOf(page) === "phone" ? page.keyboard.press("Escape") : Promise.resolve());
}
