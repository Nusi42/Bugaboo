import { t } from "@bugaboo/messages";
import { expect, type Locator, type Page } from "@playwright/test";

import { paths, projectPageAddress } from "../../src/shared/paths.ts";
import { expectGoodInEveryLook } from "./checks.ts";
import { layoutOf } from "./layout.ts";
import { inOrder } from "./sequence.ts";
import { sampleProjectName, sampleProjects, type SamplePerson } from "./sample.ts";
import { tree } from "./projects.ts";
import { shootWhole } from "./screenshots.ts";

// What the exit demo of M2 and its pictures share (m2-exit-demo.spec.ts).

/** The folder of the demo pictures inside the folder of the screenshots. */
const DEMO = "M2/demo";

/** What the pictures of a step are called: the step, the size of the screen, then the look. */
function picture(step: string, page: Page): string {
  return `${step}-${layoutOf(page) === "computer" ? "computer" : "phone"}`;
}

/** The screen is good in the four themes (it has no accessibility problem) and is in the pictures. */
export async function good(page: Page, step: string): Promise<void> {
  await expectGoodInEveryLook(page);
  await shootWhole(page, picture(step, page), DEMO);
}

/** Opens a page and waits for its heading. */
export async function open(page: Page, address: string, heading: string): Promise<void> {
  await page.goto(address);
  await expect(page.getByRole("heading", { level: 1, name: heading })).toBeVisible();
}

/** Chooses whom to check, and where when `place` is named, on the page of *Check access*. */
export async function check(page: Page, place: string | null, person: SamplePerson): Promise<void> {
  await (place === null
    ? Promise.resolve()
    : page
        .getByLabel(t("check-access.place-search"))
        .fill(place)
        .then(() => page.getByRole("radio", { name: new RegExp(place, "u") }).check()));
  await page.getByLabel(t("check-access.person-search")).fill(person.name);
  await page.getByRole("radio", { name: person.name }).check();
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
    [paths.people, paths.roles, projectPageAddress(paths.projectFields, own), `/p/${other}`].map(
      (address) => async () => {
        await page.goto(address);
        await expect(
          page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
        ).toBeVisible();
      },
    ),
  );
}

/** Closes the sheet *More* opened on a phone, so the page behind it can be used. */
export async function closeTheSheet(page: Page): Promise<void> {
  await (layoutOf(page) === "phone" ? page.keyboard.press("Escape") : Promise.resolve());
}
