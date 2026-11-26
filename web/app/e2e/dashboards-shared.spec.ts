import { t } from "@bugaboo/messages";
import { expect } from "@playwright/test";

import { dashboardAddress } from "../src/shared/places.ts";
import { lea } from "./support/access.ts";
import { ada, signInThroughApi } from "./support/accounts.ts";
import { boxOnScreen, createGuestTeam, viewOf } from "./support/dashboards.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// A shared dashboard as its viewer sees it (CFG-03, ADR-0050, decisions 2 and 3): each box runs as the viewer, a box
// the viewer may not read is left out, and the boxes under it rise, so no gap and no title tells of it.

test.use({ preparation: { run: prepareWeb } });

test("cfg_03_a_shared_dashboard_shows_the_viewer_their_own_answer_and_closes_gaps", async ({
  page,
  server,
}) => {
  const admin = await signInThroughApi(server, ada);
  const team = await createGuestTeam(admin);
  const maker = await viewOf(admin, team.id);
  await admin.dispose();
  const guest = await signInThroughApi(server, lea);
  const view = await viewOf(guest, team.id);
  await guest.dispose();
  await signIn(page, lea);
  await page.goto(dashboardAddress(team.id));
  await expect(boxOnScreen(page, team.counted)).toBeVisible();
  await expect(boxOnScreen(page, team.hidden)).toHaveCount(0);
  // The box of the guest stands on the first row of the grid: no gap tells of the one left out.
  await expect
    .poll(() =>
      boxOnScreen(page, team.counted).evaluate((box) =>
        getComputedStyle(box).getPropertyValue("--box-row").trim(),
      ),
    )
    .toBe("1");
  // The number is the guest's own answer, the issues they may see.
  await expect(boxOnScreen(page, team.counted).getByRole("cell").first()).toHaveText(
    new RegExp(` ${String(view.issues)}$`, "u"),
  );
  await expect(
    page.getByText(t("common.made-by", { person: view.maker }), { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: t("dashboards.save-copy"), exact: true }),
  ).toBeVisible();
  // Each box runs as its viewer: the guest counts fewer issues than the maker, and only the maker shares.
  expect(view.issues).toBeLessThan(maker.issues);
  await expect(page.getByRole("button", { name: t("common.share"), exact: true })).toHaveCount(0);
});
