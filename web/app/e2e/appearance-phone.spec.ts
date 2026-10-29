import { t } from "@bugaboo/messages";

import { createAdmin, forgetChoices } from "./support/accounts.ts";
import { openAppearance } from "./support/appearance.ts";
import { test } from "./support/fixtures.ts";
import { expectOneColumn, layoutOf } from "./support/layout.ts";

// *My settings › Appearance* on a phone (UX §3.3; M5 exit A14).

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test.beforeEach(async ({ server }) => {
  await forgetChoices(server);
});

test("thm_01 on a phone the choice of the colors is a list of full-width choices, not buttons of ragged widths (A14)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone stacks the choices");
  await openAppearance(page);
  // The input of a choice covers its label: the three stand in one column, as wide as each other.
  await expectOneColumn([
    page.getByRole("radio", { name: new RegExp(`^${t("theme.device")}`, "u") }),
    page.getByRole("radio", { name: t("appearance.mode.light"), exact: true }),
    page.getByRole("radio", { name: t("appearance.mode.dark"), exact: true }),
  ]);
});
