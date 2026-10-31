import { dataOf, type Schema } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";
import { createOwnField } from "./fields.ts";
import { fields, labels, projects, repairWorkflow } from "./readme-organization-data.ts";
import { inOrder } from "./sequence.ts";
import { giveOwnWorkflow } from "./workflows.ts";

// How the made-up workshop of the front page works: the workflow of its repairs, its own fields
// and its labels, set up by Marta through the API.

/** The IDs the server gave to what the issues of the workshop use, by name. */
export interface Ways {
  readonly types: Readonly<Record<string, string>>;
  readonly labels: Readonly<Record<string, string>>;
  readonly fields: { readonly customer: string; readonly frameSize: string };
  /** The choices of the field *Frame size*, by name. */
  readonly frameSizes: Readonly<Record<string, string>>;
}

/** The colors of the labels, one after the other: far apart, so the labels are told apart. */
const LABEL_COLORS: readonly Schema<"ChoiceColor">[] = [
  "cat1",
  "cat3",
  "cat5",
  "cat7",
  "cat9",
  "cat11",
];

/** The workflow of the repairs: a project's own, with a question for who does the work. */
async function setUpRepairs(admin: ApiSession): Promise<void> {
  await giveOwnWorkflow(admin, projects.repairs.shortName, {
    name: repairWorkflow.name,
    statuses: [...repairWorkflow.statuses],
    changes: repairWorkflow.changes.map((change) => ({
      from: change.from,
      to: change.to,
      button: change.button,
      rules: { ask_for: change.askForWho === true ? ["assigned-to"] : [] },
    })),
  });
}

/** The labels of the workshop, with their IDs by name. */
async function setUpLabels(admin: ApiSession): Promise<Record<string, string>> {
  const group = dataOf(
    await admin.client.POST("/api/v1/label-groups", {
      body: { name: labels.group.name, pick_one: labels.group.pickOne },
    }),
  ).id;
  const ids: Record<string, string> = {};
  await inOrder(
    [...labels.inGroup, ...labels.alone].map((name, number) => async () => {
      const inGroup = number < labels.inGroup.length;
      const made = dataOf(
        await admin.client.POST("/api/v1/labels", {
          body: {
            name,
            color: LABEL_COLORS[number % LABEL_COLORS.length] ?? null,
            ...(inGroup ? { label_group: group } : {}),
          },
        }),
      );
      ids[name] = made.id;
    }),
  );
  return ids;
}

/** Sets up how the workshop works, and gives back the IDs its issues need. */
export async function setUpWork(admin: ApiSession): Promise<Ways> {
  await setUpRepairs(admin);
  const customer = await createOwnField(admin, fields.customer.name, fields.customer.kind);
  const frameSize = await createOwnField(admin, fields.frameSize.name, fields.frameSize.kind, [
    ...fields.frameSize.choices,
  ]);
  const types = dataOf(await admin.client.GET("/api/v1/issue-types")).items;
  return {
    types: Object.fromEntries(types.map((type) => [type.name ?? "", type.id ?? ""])),
    labels: await setUpLabels(admin),
    fields: { customer: customer.id ?? "", frameSize: frameSize.id ?? "" },
    frameSizes: Object.fromEntries(
      (frameSize.choices ?? []).map((choice) => [choice.name ?? "", choice.id ?? ""]),
    ),
  };
}
