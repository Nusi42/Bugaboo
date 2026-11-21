import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { supportText } from "../src/features/about/index.ts";
import {
  aboutOf,
  downloadEveryFile,
  laterPages,
  lastPartName,
  morePartsButton,
  pressMoreWithTheKeyboard,
  readPartsToTheEnd,
  scrollUpAfterAMousePress,
} from "./support/about-words.ts";
import { bob } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// About for everybody who is signed in (COMP-01, COMP-04, ADR-0047): the version, the support period, the
// license, the parts, and the parts list and the third-party notices to download.

test.use({ preparation: { run: prepareWeb } });

test("comp_01 About shows the version the support period and the parts list to download", async ({
  page,
  server,
}) => {
  const about = await aboutOf(server);
  expect(about.files.length, "this build carries its parts list (cargo xtask build)").toBe(3);
  // Everybody who is signed in may open it: a Team Member too.
  await signIn(page, bob);
  await page.goto(paths.about);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.about") })).toBeVisible();
  await expect(page.getByText(about.version, { exact: true })).toBeVisible();
  await expect(page.getByText(supportText(about.support_period), { exact: true })).toBeVisible();
  await expect(page.getByText(about.license, { exact: true })).toBeVisible();
  // The parts, by name, version and license.
  const parts = page.getByRole("listbox", { name: t("about.parts") });
  await expect(parts).toBeVisible();
  await expect(parts.getByRole("option").first()).toBeVisible();
  // Only a page of parts is read at first: *Show more* reads the rest.
  await readPartsToTheEnd(page, parts, about, await lastPartName(server));
  await expectGoodInEveryLook(page);
  await downloadEveryFile(page, about);
});

test("comp_15 Show more stays in view for the keyboard and leaves a mouse where it scrolled", async ({
  page,
  server,
}) => {
  expect(laterPages(await aboutOf(server)), "the parts take three pages at least").toBeGreaterThan(
    1,
  );
  await signIn(page, bob);
  await page.goto(paths.about);
  const parts = page.getByRole("listbox", { name: t("about.parts") });
  await expect(parts.getByRole("option").first()).toBeVisible();
  // The keyboard: the button is in view when its page is there and the list has measured its rows.
  await pressMoreWithTheKeyboard(page, parts);
  await expect(morePartsButton(page)).toBeInViewport();
  // The mouse: the page stays where the person scrolled it to while the list measures the rows it draws.
  const scrolled = await scrollUpAfterAMousePress(page);
  expect(scrolled.scrolled).toBeLessThan(scrolled.pressed);
  expect(scrolled.settled).toBe(scrolled.scrolled);
  await expect(morePartsButton(page)).not.toBeInViewport();
});
