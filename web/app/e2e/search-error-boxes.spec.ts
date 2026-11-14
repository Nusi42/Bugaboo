import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, refusal, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";

// The box of a search that takes too long (UX §4.2, §6.1; M5 exit): the same words and the same
// box on a phone and on a computer, and no way to "show the mistake" there is none to show.

test.use({ preparation: { run: prepareLook } });

test("ux_6_1 a search that takes too long says so, and offers no mistake to show", async ({
  page,
}) => {
  await signIn(page, ada);
  // The answer has the code of the search that took too long; a 4xx status is what the tests of refusals may log (a 503 would count as a defect of the page).
  await page.route(/\/api\/v1\/search\/issues\/(count|groups)/u, (route) =>
    route.fulfill(refusal(408, "search.too-slow")),
  );
  await page.goto(projectIssuesAddress("WEB"));
  await expect(page.getByRole("alert")).toContainText(t("error.search.too-slow"));
  await expect(page.getByRole("button", { name: t("search.show-mistake") })).toHaveCount(0);
});
