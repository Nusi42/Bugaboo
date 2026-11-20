import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { aboutOf, downloadEveryFile, supportWords } from "./support/about-words.ts";
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
  await expect(page.getByText(supportWords(about.support_period), { exact: true })).toBeVisible();
  await expect(page.getByText(about.license, { exact: true })).toBeVisible();
  // The parts, by name, version and license.
  await expect(page.getByRole("listbox", { name: t("about.parts") })).toBeVisible();
  await expect(page.getByRole("option").first()).toBeVisible();
  await expectGoodInEveryLook(page);
  await downloadEveryFile(page, about);
});
