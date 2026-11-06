import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { roleId, roleName, setFieldLevel, signedInAdmin } from "./access.ts";
import type { ApiSession } from "./api.ts";
import { estimate } from "./issue-time.ts";
import { issueOf, prepareWeb } from "./issues.ts";
import { addToTheFormOfWeb, saveLayoutSections } from "./layouts.ts";
import type { Instance } from "./server.ts";
import { startTimerOn } from "./time.ts";

/** The three lengths of time in the order the layout of the tests places them: the reverse of the usual. */
export const lengthsReversed = ["time-spent", "time-left", "estimated-time"] as const;

/** The names of the three lengths, as the rows of the Details and the boxes carry them. */
export const lengthNames = {
  "time-spent": t("issue-field.time-spent"),
  "time-left": t("issue-field.time-left"),
  "estimated-time": t("issue-field.estimated-time"),
} as const;

/**
 * The project WEB with a layout of its own for the issue page that places the three lengths of
 * time after *Status* and before *Assigned to*, in the reverse of the usual order, for every role.
 * The issue page then shows them where the layout says. The form of a new issue asks for the
 * estimate too.
 */
export async function prepareTimeLayout(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  const created = dataOf(
    await admin.client.POST("/api/v1/layouts", { body: { kind: "issue-page", project: "WEB" } }),
  );
  const slot = created.layout;
  if (slot === undefined || slot === null) {
    throw new TypeError("The layout was not created.");
  }
  const roles = created.roles.map((role) => role.id);
  const words: readonly Schema<"EditorFieldBody">["field"][] = [
    "description",
    "status",
    ...lengthsReversed,
    "assigned-to",
    "priority",
    "reported-by",
  ];
  await saveLayoutSections(admin, slot, [
    { tab: "", title: "", fields: words.map((field) => ({ field, roles })) },
  ]);
  await addToTheFormOfWeb(admin, ["estimated-time"]);
  await admin.dispose();
}

/**
 * A timer that runs on *WEB-1*, and the layout of the issue page of the project changed to place
 * none of the three lengths of time.
 */
export async function runTimerWithoutLengthsInTheLayout(admin: ApiSession): Promise<void> {
  await startTimerOn(admin, await issueOf(admin, "WEB-1"));
  const editor = dataOf(
    await admin.client.GET("/api/v1/layouts/editor", {
      params: { query: { kind: "issue-page", project: "WEB" } },
    }),
  );
  const slot = editor.layout;
  if (slot === undefined || slot === null) {
    throw new TypeError("The project has no layout of its own.");
  }
  const lengths: readonly string[] = lengthsReversed;
  const sections = editor.sections.map((section: Schema<"EditorSectionBody">) => ({
    tab: section.tab,
    title: section.title,
    fields: section.fields.filter(
      (each: Schema<"EditorFieldBody">) => !lengths.includes(each.field),
    ),
  }));
  await saveLayoutSections(admin, slot, sections);
}

/**
 * Runs `run` while the Team members have this level for *Estimated time* and *WEB-1* has an
 * estimate (an issue with none has no estimate to see), and gives them the level they have by
 * default again afterwards, whatever `run` does.
 */
export async function withEstimateLevel(
  server: Instance,
  level: "can-see" | "hidden",
  run: () => Promise<void>,
): Promise<void> {
  const admin = await signedInAdmin(server);
  const team = await roleId(admin, roleName.teamMember);
  await estimate(admin, await issueOf(admin, "WEB-1"), 90);
  await setFieldLevel(admin, { role: team, field: "estimated-time" }, level);
  try {
    await run();
  } finally {
    await setFieldLevel(admin, { role: team, field: "estimated-time" }, "can-change");
    await admin.dispose();
  }
}
