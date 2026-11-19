import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { adasKey, aProgramAsks } from "./support/agent-requests.ts";
import { prepareWebWithAgents } from "./support/agents-scene.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { setBox } from "./support/list-keys.ts";

// *Create several agents* and the requests of programs (AI-30, AI-37, ADR-0029 amendment M9): a list
// of agents with one key each and the password once, the keys shown once with *Copy all keys*; a
// program asks for agents over `/mcp`, and Ada approves one request and declines another. The same
// specs run on a computer and on a phone.

test.use({ preparation: { run: prepareWebWithAgents } });

/** Names of agents nobody has used on this server yet: agent names are taken for good. */
let lists = 0;
function names(stem: string): readonly string[] {
  lists += 1;
  return [`${stem} planner ${lists}`, `${stem} builder ${lists}`];
}

test("ai_37 several agents are created at once and their keys copied", async ({ page }) => {
  const wanted = names("List");
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  await page.getByRole("button", { name: t("agents.several"), exact: true }).click();
  await page.getByLabel(t("agents.several-names")).fill(wanted.join("\n"));
  await page.getByLabel(t("field.confirm-password")).fill(ada.password);
  await page
    .getByRole("button", { name: t("agents.several-confirm", { count: wanted.length }) })
    .click();
  const shown = page.getByRole("list", { name: t("agents.several-keys") });
  await expect(shown.getByRole("listitem")).toHaveCount(wanted.length);
  await Promise.all(
    wanted.map((name) => expect(shown.getByText(name, { exact: true })).toBeVisible()),
  );
  await expect(page.getByRole("button", { name: t("agents.several-copy-all") })).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("access-keys.done") }).click();
  await expect(shown).toBeHidden();
});

test("ai_37 a request of a program is approved and declined", async ({ page, server }) => {
  const key = await adasKey(server);
  const approved = names("Asked");
  const declined = names("Declined");
  await aProgramAsks(server, key, approved);
  await aProgramAsks(server, key, declined);
  await signIn(page, ada);
  await page.goto(paths.myAgents);
  const requests = page.getByRole("list", { name: t("agents.requests") });
  await expect(requests.getByRole("listitem")).toHaveCount(2);
  await expectGoodInEveryLook(page);
  // The newest request is first: decline it.
  await requests
    .getByRole("listitem")
    .first()
    .getByRole("button", { name: t("agents.decline") })
    .click();
  await expect(requests.getByRole("listitem")).toHaveCount(1);
  // Tick the other and approve it with the password.
  await setBox(requests.getByRole("checkbox"), true);
  await page.getByLabel(t("field.confirm-password")).fill(ada.password);
  await page.getByRole("button", { name: t("agents.requests-approve", { count: 1 }) }).click();
  // The approved request now waits for its keys, and its agents are on the page.
  await expect(requests.getByRole("checkbox")).toHaveCount(0);
  await Promise.all(
    approved.map((name) => expect(page.getByText(name, { exact: true }).first()).toBeVisible()),
  );
});
