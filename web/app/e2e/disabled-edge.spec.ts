import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import { blockOf, openSimple } from "./support/workflows-pages.ts";
import { inEveryLook, resolved } from "./support/styles.ts";

// A button that cannot be pressed has the edge of the theme file, which STY-CONTRAST holds at a
// ratio in every theme (the final check of M4: in Dusk and Midnight it was about 1.24 to 1 against
// the page). A look without an edge, a quiet button, keeps none when it is disabled (the review
// of WP-18). The test only says which color the browser draws; the ratio is not worked out here.

test.use({ preparation: { run: prepareWorkflows } });

test("ux_07 the edge of a disabled button is the one of the theme file, in every theme", async ({
  page,
}) => {
  await signIn(page, ada);
  await openSimple(page);
  const publish = page.getByRole("button", { name: t("workflows.publish"), exact: true });
  await expect(publish).toBeDisabled();
  await inEveryLook(page, async (look) => {
    const edge = await resolved(page, "border-top-color", "var(--fg-muted)");
    await expect(publish, look).toHaveCSS("border-top-color", edge);
  });
});

test("ux_07 a quiet button that cannot be pressed has no edge", async ({ page }) => {
  await signIn(page, ada);
  await openSimple(page);
  const status = await page.getByRole("heading", { level: 3 }).first().innerText();
  const up = blockOf(page, status).getByRole("button", {
    name: t("workflows.status-up", { name: status }),
    exact: true,
  });
  await expect(up).toBeDisabled();
  await expect(up).toHaveCSS(
    "border-top-color",
    await resolved(page, "border-top-color", "transparent"),
  );
});
