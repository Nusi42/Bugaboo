import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { createAppSession, issueIdOf } from "./support/runs.ts";

// The history of an issue says which AI app a change went through: "Ada Admin via Claude Code" (AI-23).

test.use({ preparation: { run: prepareWeb } });

test("ai_23 the history of an issue names the app a change went through", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await createAppSession(server, admin, [
    { issue: { id: await issueIdOf(admin, "WEB-1"), key: "WEB-1" }, version: 1 },
  ]);
  await admin.dispose();
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const said = t("person.via-app", { who: "Ada Admin", app: "Claude Code" });
  await expect(page.getByText(said).first()).toBeVisible();
});
