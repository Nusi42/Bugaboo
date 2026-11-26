import { ada } from "./support/accounts.ts";
import { cardOf, cards, openBoard, prepareBoard } from "./support/board.ts";
import { resetBoard } from "./support/board-reset.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";

// *Enter* on the title of a card opens the issue and leaves the page usable (PLAN-06, UX §4.6): no drag session starts, so
// nothing is inert and the next Tab takes the focus on.

test.use({ preparation: { run: prepareBoard } });

test.beforeEach(async ({ server }) => {
  await resetBoard(server);
});

test("plan_06_enter_on_the_title_of_a_card_opens_the_issue_and_leaves_the_page_usable", async ({
  page,
}) => {
  await signIn(page, ada);
  await openBoard(page);
  await partIn(cardOf(page, cards.working), "board-card.title").getByRole("link").focus();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(new RegExp(`/issues/${cards.working}`, "u"));
  await expect(page.locator("main[inert], main [inert]")).toHaveCount(0);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(page.locator("main").locator(":focus")).toHaveCount(1);
});
