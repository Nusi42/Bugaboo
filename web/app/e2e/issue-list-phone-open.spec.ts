import { issueAddress, projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expectNoSidewaysScroll } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { rowOf } from "./support/list-look.ts";
import { preparePanel } from "./support/list-panel.ts";
import { issueBackOf, layoutOf } from "./support/layout.ts";

// On a phone an issue opened from a list is its full page with Back to the list; the side panel is
// for computers only (UX §3.4, maintainer decision D1 of the M5 exit wave, finding A1). The person here
// has chosen the side panel, which a phone cannot show next to a list.

test.use({ preparation: { run: preparePanel } });

test("ux_3_4 on a phone an issue of the list opens as its page, and Back returns to the same search (A1)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone opens the full page");
  await signIn(page, ada);
  const search = "s=priority+is+not+low&group=priority";
  await page.goto(`${projectIssuesAddress("WEB")}?${search}`);
  await rowOf(page, webIssues.login).getByRole("link", { name: webIssues.login }).click();
  await expect(page).toHaveURL(new RegExp(`${issueAddress("WEB-1")}$`, "u"));
  await expect(page.getByRole("heading", { level: 1, name: webIssues.login })).toBeVisible();
  // The top bar of every screen, with Back and the issue ID (L1).
  await expect(page.getByRole("banner")).toHaveCount(1);
  await expectNoSidewaysScroll(page);
  await issueBackOf(page).click();
  await expect.poll(() => new URL(page.url()).pathname).toBe(projectIssuesAddress("WEB"));
  expect(new URL(page.url()).searchParams.get("s")).toBe("priority is not Low");
  expect(new URL(page.url()).searchParams.get("open")).toBeNull();
});
