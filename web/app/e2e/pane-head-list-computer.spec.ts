import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareManyWeb } from "./support/issues.ts";
import { expectStill, movementWhile, partLandmark } from "./support/movement.ts";
import { foldsWith, LOW_WINDOW, wheelOver } from "./support/pane-head.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The head of a list (the toolbar, the search bar, the options and the counts) folds to its toolbar as the rows scroll
// and opens again at their top (UX §6.5); the head of the columns goes up with it, and nothing else moves.

/** More rows than the low window shows. */
const MANY = 40;

test.use({ preparation: { run: (server) => prepareManyWeb(server, MANY) } });

test("ux_01 the head of a list folds as its rows scroll and nothing else moves in any frame", async ({
  page,
}) => {
  await page.setViewportSize(LOW_WINDOW);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const rows = page.locator("[data-part='list-scroll']");
  await expect(rows).toBeVisible();
  await page.waitForLoadState("networkidle");
  const movement = await movementWhile(
    page,
    [
      partLandmark("the heading of the card", "card.heading"),
      partLandmark("the toolbar", "list-toolbar"),
      partLandmark("the rows", "list-scroll"),
    ],
    () => wheelOver(page, rows),
  );
  // The head folds and opens again with the scroll; the heading and the toolbar, its first line, stay where they are.
  expectStill(movement, { scrolls: ["list-scroll"] });
  // The whole head (the search bar and the options of the list are in it once the list is read) folds and opens.
  await foldsWith(page, rows);
});
