import { dataOf } from "@bugaboo/api-client";
import { parseWindowSizes } from "@bugaboo/ui";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada, forgetChoices } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { apiOf, signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareWeb } from "./support/issues.ts";
import { openWindowReady, pressSizeKey, sizesSaved, windowSize } from "./support/windows.ts";

// The sizes of the windows are one setting of the person (UX §6.3) and the person can have it
// open in several tabs and on several devices: a size set elsewhere is never undone by a tab that
// read the setting before (M5-F97). A phone has a full screen, not a window.

test.use({ preparation: { run: prepareWeb } });
test.beforeEach(async ({ page, server }) => {
  test.skip(layoutOf(page) !== "computer", "a phone has a full screen, not a window");
  await forgetChoices(server);
});

const otherForm = { width: 640, height: 480 };

test("m5_f97 a size set in another tab for another form stays when this tab resizes its form", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openWindowReady(page);
  const before = await windowSize(page);
  // Another tab sets the size of another form after this tab read the setting.
  const other = apiOf(page, server);
  await other.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: "window.sizes" } },
    body: { value: JSON.stringify({ "other-form": otherForm }), version: 0 },
  });
  await sizesSaved(page, 2, async () => {
    await pressSizeKey(page, "ArrowRight");
    await expect.poll(async () => (await windowSize(page)).width).toBeGreaterThan(before.width);
  });
  const after = await windowSize(page);
  const kept = dataOf(
    await other.client.GET("/api/v1/me/settings", { params: { query: { topic: "window.sizes" } } }),
  );
  const sizes = parseWindowSizes(String(kept.settings[0]?.value));
  expect(sizes["other-form"]).toEqual(otherForm);
  expect(sizes["new-issue"]).toEqual(after);
  // A second tab finds the size this tab gave the form.
  const second = await page.context().newPage();
  await second.goto(projectIssuesAddress("WEB"));
  await openWindowReady(second);
  await expect.poll(() => windowSize(second)).toEqual(after);
});
