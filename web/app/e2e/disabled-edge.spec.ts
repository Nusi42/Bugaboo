import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWorkflows } from "./support/workflows.ts";
import { blockOf, openSimple } from "./support/workflows-pages.ts";
import { inOrder } from "./support/sequence.ts";
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

test("ux_07 a main button that cannot be pressed is flat and quiet, not selected and not outlined, in every theme", async ({
  page,
}) => {
  // The review of M5 (S20): it looked like an outlined button with grey words; with the soft accent fill it
  // looked like a selected segment. It is sunken, with quiet words and the edge of a button that cannot be
  // pressed (tested above): the fill of neither a chosen thing (`bg-selected`, `accent-soft`) nor a button.
  await signIn(page, ada);
  await openSimple(page);
  const publish = page.getByRole("button", { name: t("workflows.publish"), exact: true });
  await expect(publish).toBeDisabled();
  await inEveryLook(page, async (look) => {
    const sunken = await resolved(page, "background-color", "var(--bg-sunken)");
    await expect(publish, look).toHaveCSS("background-color", sunken);
    await expect(publish, look).toHaveCSS(
      "color",
      await resolved(page, "color", "var(--fg-muted)"),
    );
    await inOrder(
      ["var(--accent-soft)", "var(--bg-selected)", "var(--bg-raised)"].map((chosen) => async () => {
        expect(sunken, `${look}: not ${chosen}`).not.toBe(
          await resolved(page, "background-color", chosen),
        );
      }),
    );
  });
});
