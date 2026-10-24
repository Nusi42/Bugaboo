import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { edgeContrast } from "./support/edge-contrast.ts";
import { signIn } from "./support/flows.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import { openSimple } from "./support/workflows-pages.ts";
import { inEveryLook } from "./support/styles.ts";

// A button that cannot be pressed still has an edge that can be seen, in every theme (the final
// check of M4: in Dusk and Midnight it was about 1.24 to 1 against the page).

/** The least contrast the edge of a disabled button has against the page it stands on. */
const LEAST_EDGE_CONTRAST = 1.8;

test.use({ preparation: { run: prepareWorkflows } });

test("ux_07 the edge of a disabled button can be seen in every theme", async ({ page }) => {
  await signIn(page, ada);
  await openSimple(page);
  const publish = page.getByRole("button", { name: t("workflows.publish"), exact: true });
  await expect(publish).toBeDisabled();
  await inEveryLook(page, async (look) => {
    const contrast = await edgeContrast(publish);
    expect(contrast, look).toBeGreaterThanOrEqual(LEAST_EDGE_CONTRAST);
  });
});
