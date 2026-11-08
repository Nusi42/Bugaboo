import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { openAssistant, setUpAssistant } from "./support/assistant.ts";
import { startFakeModel } from "./support/fake-model.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";

// The page is never crushed by the assistant (M7 WP-10, UX §4.12): it docks beside the page only where the
// page keeps the room it has at the computer layout; narrower, it opens as a layer from the end and the page
// keeps its width. The breakpoint is the token `bp-assistant-dock`.

test.use({ preparation: { run: prepareWeb } });

/** How wide the text of the description of the issue is drawn. */
function descriptionWidth(page: Parameters<typeof openAssistant>[0]): Promise<number> {
  return page
    .getByText("Steps:", { exact: true })
    .evaluate((element: HTMLElement) => element.getBoundingClientRect().width);
}

/** The assistant docks at this width, and the dock is on the page. */
const DOCKED = '[data-part="assistant-dock"]';

test("ai_10 at the computer width the page keeps its width and at a wide screen the assistant docks", async ({
  page,
  server,
}) => {
  const model = await startFakeModel();
  await setUpAssistant(server, model);
  await signIn(page, ada);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(issueAddress("WEB-1"));
  const closed = await descriptionWidth(page);
  await openAssistant(page);
  // A layer: nothing docks, and the text of the issue is as wide as before.
  await expect(page.locator(DOCKED)).toHaveCount(0);
  expect(await descriptionWidth(page)).toBeGreaterThanOrEqual(closed - 1);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog", { name: t("assistant.title") })).toBeHidden();
  // A wide screen: the assistant stands beside the page, which keeps at least the room it has at 1280.
  await page.setViewportSize({ width: 1800, height: 800 });
  await page.goto(issueAddress("WEB-1"));
  await openAssistant(page);
  await expect(page.locator(DOCKED)).toHaveCount(1);
  expect(await descriptionWidth(page)).toBeGreaterThanOrEqual(closed - 1);
  await model.close();
});
