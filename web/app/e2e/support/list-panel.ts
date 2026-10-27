import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { signedInAdmin } from "./access.ts";
import { openIssuesIn } from "./issues.ts";
import { layoutOf } from "./layout.ts";
import { prepareLook } from "./list-look.ts";
import { type Instance } from "./server.ts";

/** The list of the look, for a person who opens issues in the side panel. */
export async function preparePanel(server: Instance): Promise<void> {
  await prepareLook(server);
  const admin = await signedInAdmin(server);
  await openIssuesIn(admin, "panel");
  await admin.dispose();
}

/** The button that closes the issue open next to the list: the panel's own, or Back to list on a phone. */
export function panelCloseOf(page: Page): Locator {
  return layoutOf(page) === "phone"
    ? page.getByRole("button", { name: t("focus.back"), exact: true })
    : page.getByRole("complementary").getByRole("button", { name: t("panel.close"), exact: true });
}
