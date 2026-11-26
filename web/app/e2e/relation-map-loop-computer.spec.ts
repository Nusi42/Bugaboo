import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { ada, signInThroughApi, type Instance } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { createIssue, prepareWeb } from "./support/issues.ts";
import { kindOf, kindWords, openMap, relate } from "./support/relation-map.ts";

// A loop on the relation map (ISS-16, UX §4.15): a loop of *blocks* stored before loops were
// refused is flagged with its sentence.

/** The two issues of the loop, made after WEB-3. */
const loop = { first: "WEB-4", second: "WEB-5" } as const;

/** A loop of *blocks*: the kind loses its date effect, so the loop can be stored as one left from before. */
async function prepareLoop(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signInThroughApi(server, ada);
  const blocks = await kindOf(admin, kindWords.blocks);
  const kinds = dataOf(await admin.client.GET("/api/v1/relation-kinds"));
  const version = kinds.items.find((kind) => kind.id === blocks)?.version ?? 0;
  dataOf(
    await admin.client.PATCH("/api/v1/relation-kinds/{relation_kind}", {
      params: { path: { relation_kind: blocks } },
      body: { version, date_effect: "none" },
    }),
  );
  await createIssue(admin, "WEB", "The first of a loop");
  await createIssue(admin, "WEB", "The second of a loop");
  await relate(admin, loop.first, kindWords.blocks, loop.second);
  await relate(admin, loop.second, kindWords.blocks, loop.first);
  await admin.dispose();
}

test.use({ preparation: { run: prepareLoop } });

test("iss_16_a_loop_is_flagged_with_its_sentence", async ({ page }) => {
  await signIn(page, ada);
  await openMap(page, loop.first);
  const lead = t("relation-map.lead-next", {
    lead: t("relation-map.lead-first", { key: loop.first, word: kindWords.blocks }),
    key: loop.second,
    word: kindWords.blocks,
  });
  const sentence = t("relation-map.sentence", { lead, key: loop.first });
  await expect(page.getByText(t("relation-map.loop", { sentence }), { exact: true })).toBeVisible();
});
