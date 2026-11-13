import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { landmarkOf, movementWhile } from "./support/movement.ts";
import { readEverythingAgain } from "./support/read-again.ts";
import { issueAddress } from "../src/shared/paths.ts";

// A refusal stays on screen while it is read again (UX §6.4, M8 WP-13): the query cache forgets the
// refusal of a read that has no answer the moment it reads it again, and the page must not.

test.use({ preparation: { run: prepareWeb } });

test("ux_01 a page that is not found keeps saying so while it is read again", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-999"));
  const heading = page.getByRole("heading", { level: 1, name: t("page.not-found.title") });
  await expect(heading).toBeVisible();
  const said = await landmarkOf(heading, "Not found");
  const read = await movementWhile(page, [said], () => readEverythingAgain(page));
  expect({ moved: read.moved, vanished: read.vanished }).toEqual({ moved: [], vanished: [] });
});
