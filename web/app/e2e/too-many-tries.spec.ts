import { t } from "@bugaboo/messages";

import { ada, makeAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, sendPassword } from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

test.use({
  // Three wrong tries are allowed per minute: the fourth has to wait.
  serverSettings: { BUGABOO_SIGN_IN_FAILURES_PER_MINUTE: "3" },
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});

test("sec_14 too many wrong passwords make the person wait, even for the right password", async ({
  page,
}) => {
  const wrong = t("error.auth.wrong-credentials");
  await sendPassword(page, ada, "not-the-password");
  await expect(page.getByRole("alert")).toHaveText(wrong);
  await sendPassword(page, ada, "not-the-password");
  await expect(page.getByRole("alert")).toHaveText(wrong);
  await sendPassword(page, ada, "not-the-password");
  await expect(page.getByRole("alert")).toHaveText(wrong);

  await sendPassword(page, ada, "not-the-password");
  await expect(page.getByRole("alert")).toHaveText(t("error.rate-limit.exceeded"));
  // The account waits, even for its right password: nobody can guess their way in by trying on.
  await sendPassword(page, ada);
  await expect(page.getByRole("alert")).toHaveText(t("error.rate-limit.exceeded"));
  await expect(page).toHaveURL(addressEndingAt(paths.signIn));
});
