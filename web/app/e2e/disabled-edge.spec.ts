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

test("ux_07 the edge of a main button that cannot be pressed is its grey fill, in every theme", async ({
  page,
}) => {
  await signIn(page, ada);
  await openSimple(page);
  const publish = page.getByRole("button", { name: t("workflows.publish"), exact: true });
  await expect(publish).toBeDisabled();
  await inEveryLook(page, async (look) => {
    const edge = await resolved(page, "border-top-color", "var(--disabled-fill)");
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
  // looked like a selected segment, and sunken or faded it read like an enabled outlined button in the dark
  // themes. It is a grey chip with quiet words, not faded: the fill of neither a chosen thing (`bg-selected`,
  // `accent-soft`) nor a button (`bg-raised`).
  await signIn(page, ada);
  await openSimple(page);
  const publish = page.getByRole("button", { name: t("workflows.publish"), exact: true });
  await expect(publish).toBeDisabled();
  await inEveryLook(page, async (look) => {
    const grey = await resolved(page, "background-color", "var(--disabled-fill)");
    await expect(publish, look).toHaveCSS("background-color", grey);
    // Not faded: the grey chip is told from the surface by its own color, not by being thin.
    await expect(publish, look).toHaveCSS("opacity", "1");
    await expect(publish, look).toHaveCSS(
      "color",
      await resolved(page, "color", "var(--on-disabled)"),
    );
    await inOrder(
      ["var(--accent-soft)", "var(--bg-selected)", "var(--bg-raised)"].map((chosen) => async () => {
        expect(grey, `${look}: not ${chosen}`).not.toBe(
          await resolved(page, "background-color", chosen),
        );
      }),
    );
  });
});

test("ux_07 under forced colors a main button that cannot be pressed has the face and the gray words of the system", async ({
  page,
}) => {
  await page.emulateMedia({ forcedColors: "active" });
  await signIn(page, ada);
  await openSimple(page);
  const publish = page.getByRole("button", { name: t("workflows.publish"), exact: true });
  await expect(publish).toBeDisabled();
  await expect(publish).toHaveCSS(
    "background-color",
    await resolved(page, "background-color", "ButtonFace"),
  );
  await expect(publish).toHaveCSS("color", await resolved(page, "color", "GrayText"));
});
