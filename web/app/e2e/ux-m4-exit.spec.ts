import { t } from "@bugaboo/messages";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";
import { signIn } from "./support/flows.ts";
import { newIssueDialog, openNewIssueDialog, prepareWeb } from "./support/issues.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { openAction, tickIssues } from "./support/many-changes-pages.ts";
import { boxOf, expectInsideTheScreen, footerButtons, linesOf } from "./support/windows.ts";

// What the UX review of the M4 exit asked for on every screen (M4 WP-14, UX A1 to A4): windows and
// questions stay inside the screen, the one filled button of a question or a form is at its end
// (below the others on a phone), the text buttons are at its start, and a date or a time is never
// broken over two lines.

/** A check that does not need the page, as a step of a layout. */
function together(check: () => void): () => Promise<void> {
  return () => {
    check();
    return nothing();
  };
}

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("ux_a1 the new-issue window stays inside the screen, with its main button at the end", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await expectInsideTheScreen(page, newIssueDialog(page));
  const { cancel, another, create } = await footerButtons(page);
  // On a computer the plain Cancel stands at the start and the filled Create at the end.
  const atTheEnd = (): void => {
    expect(cancel.x).toBeLessThan(another.x);
    expect(another.x).toBeLessThan(create.x);
    expect(create.x + create.width).toBeGreaterThanOrEqual(another.x + another.width);
  };
  await onThisLayout(page, {
    computer: together(atTheEnd),
    tablet: together(atTheEnd),
    // On a phone the buttons are under each other, the filled one first and as wide as the others.
    phone: () => {
      expect(create.y).toBeLessThan(another.y);
      expect(another.y).toBeLessThan(cancel.y);
      expect(Math.abs(create.width - cancel.width)).toBeLessThanOrEqual(1);
      return nothing();
    },
  });
});

test("ux_a2 a question of many changes is inside the screen, and its filled button is the last", async ({
  page,
}) => {
  await signIn(page, ada);
  await tickIssues(page, ["WEB-1", "WEB-2"]);
  const dialog = await openAction(page, "change", 2);
  await expectInsideTheScreen(page, dialog);
  const main = dialog.getByRole("button", {
    name: t("many-changes.apply", { count: 2 }),
    exact: true,
  });
  const cancel = dialog.getByRole("button", { name: t("common.cancel"), exact: true });
  await expect(main).toHaveAttribute("data-variant", /primary/u);
  await expect(cancel).not.toHaveAttribute("data-variant", /primary/u);
  const [mainBox, cancelBox] = [await boxOf(main), await boxOf(cancel)];
  const startsFirst = (): void => {
    expect(cancelBox.x).toBeLessThan(mainBox.x);
  };
  await onThisLayout(page, {
    computer: together(startsFirst),
    tablet: together(startsFirst),
    phone: () => {
      expect(mainBox.y).toBeLessThan(cancelBox.y);
      return nothing();
    },
  });
});

test("ux_a4 a date or a time of the issue page is on one line", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  // The moment a change was made, in the history under the issue.
  const date = partIn(page, "history.when").first();
  await expect(date).toBeVisible();
  expect(await linesOf(date)).toBe(1);
});
