import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { issueAddress, panelAddress } from "./support/addresses.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { chooseStatus, prepareWeb } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { serverWord } from "./support/server-words.ts";

// The frames an issue opens in (UX §3.2, §3.3, §4.4, M3 WP-13): the side panel, the focus view and
// the bar of a phone each say once what they have to say, and the buttons do what they name.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ux_03 the side panel has one way to close and says the ID once", async ({ page }) => {
  test.skip(layoutOf(page) !== "computer", "the panel sits next to the list on a computer");
  await signIn(page, ada);
  await page.goto(panelAddress("WEB", "WEB-2"));
  const panel = page.getByRole("complementary", { name: "WEB-2", exact: true });
  await expect(panel.getByRole("button", { name: t("panel.close"), exact: true })).toHaveCount(1);
  await expect(panel.getByRole("button", { name: t("common.close"), exact: true })).toHaveCount(0);
  // The ID is the heading of the panel; the button beside it names what it does.
  await expect(panel.getByText("WEB-2", { exact: true })).toHaveCount(1);
  await expect(
    panel.getByRole("button", { name: t("issues.copy-link-of", { key: "WEB-2" }), exact: true }),
  ).toBeVisible();
});

test("ux_04 copying the address is announced", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await page
    .getByRole("button", { name: t("issues.copy-link-of", { key: "WEB-2" }), exact: true })
    .click();
  await expect(page.getByRole("status").filter({ hasText: t("issues.link-copied") })).toHaveCount(
    1,
  );
});

test("ux_04 on a phone the panel says Back to list, and the bar goes to the status and the comment box", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar");
  await signIn(page, ada);
  await page.goto(panelAddress("WEB", "WEB-2"));
  await expect(page.getByRole("button", { name: t("focus.back"), exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: t("panel.close"), exact: true })).toHaveCount(0);
  await page
    .getByRole("toolbar", { name: t("issues.bar"), exact: true })
    .getByRole("button", { name: t("issue-field.status"), exact: true })
    .click();
  // The list of the big status button opens, as it does for the key s.
  await expect(
    page.getByRole("dialog", { name: t("issues.change-status"), exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await page
    .getByRole("toolbar", { name: t("issues.bar"), exact: true })
    .getByRole("button", { name: t("issues.comment"), exact: true })
    .click();
  await expect(page.getByRole("textbox", { name: t("comments.write"), exact: true })).toBeFocused();
});

/** Where a place of the page starts and ends, from the top of the window. */
function edgeOf(place: Locator): Promise<{ readonly top: number; readonly bottom: number }> {
  return place.evaluate((element) => {
    const { top, bottom } = element.getBoundingClientRect();
    return { top, bottom };
  });
}

test("ux_04 on a phone nothing of the page is under a bar, also when the issue is done and a note is up", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the bar");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-3"));
  await chooseStatus(page, serverWord("workflow-change.mark-as-done"));
  // The note that says what changed, with Undo, is up now: it stands between the page and the bars.
  const note = page.getByRole("status").filter({ hasText: t("common.undo") });
  await expect(note).toBeVisible();
  const bar = page.getByRole("toolbar", { name: t("issues.bar"), exact: true });
  // The bar is in a landmark of its own, not outside every landmark of the page.
  await expect(
    page.getByRole("region", { name: t("issues.bar"), exact: true }).getByRole("toolbar"),
  ).toBeVisible();
  const pageEdge = await edgeOf(page.locator("main"));
  const noteEdge = await edgeOf(note);
  const barEdge = await edgeOf(bar);
  // From the top: the page, then the note, then the bar. None covers another.
  expect(pageEdge.bottom).toBeLessThanOrEqual(noteEdge.top);
  expect(noteEdge.bottom).toBeLessThanOrEqual(barEdge.top);
  // At the first place the page stands, no button is half under anything: axe checks the size of
  // every target that is partly covered.
  await expectGoodInEveryLook(page);
});
