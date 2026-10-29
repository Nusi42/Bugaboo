import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createProject, openMove } from "./support/projects.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";
import { inEveryLook, resolved } from "./support/styles.ts";

// M5 exit, package T (second picture review): a radio is drawn by the theme, not by the browser's own blue
// or grey dot (a switched-off look in Dusk and Midnight): an outlined circle with the edge of a control, and
// the accent when it is chosen. The box to tick of a row has a stronger edge on a tint.

test.describe("the radios of a dialog", () => {
  test.use({
    preparation: {
      run: async (server) => {
        const admin = await createAdmin(server);
        const website = await createProject(admin, "WEB", "Website");
        await createProject(admin, "DOCS", "Documents", website);
        await createProject(admin, "OPS", "Operations");
        await admin.dispose();
      },
    },
  });

  test("ux_7_4 a radio is an outlined circle, and the accent when it is chosen, in every theme", async ({
    page,
  }) => {
    await signIn(page, ada);
    const dialog = await openMove(page, addressInProject(paths.projectSettings, "DOCS"));
    const chosen = dialog.getByRole("radio", { name: t("projects.picker-top") });
    const radio = dialog.getByRole("radio", { name: /Operations/u });
    await expect(chosen).toBeChecked();
    await expect(radio).not.toBeChecked();
    await inEveryLook(page, async (look) => {
      await expect(radio, look).toHaveCSS("appearance", "none");
      await expect(radio, look).toHaveCSS(
        "border-top-color",
        await resolved(page, "border-top-color", "var(--border-control)"),
      );
      await expect(radio, look).toHaveCSS(
        "border-top-left-radius",
        await resolved(page, "border-top-left-radius", "var(--r-pill)"),
      );
    });
    await radio.check();
    await inEveryLook(page, async (look) => {
      await expect(radio, look).toHaveCSS(
        "background-color",
        await resolved(page, "background-color", "var(--accent)"),
      );
    });
  });
});
