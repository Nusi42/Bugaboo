import { dataOf } from "@bugaboo/api-client";

import { versionOf, type MadeIssue } from "./issues.ts";
import { comment } from "./notifications.ts";
import { idOf, type People } from "./readme-company.ts";
import {
  cargoBike,
  cargoBikeComments,
  cargoBikeHub,
  cargoBikeRotor,
  type Job,
  projects,
  repairWorkflow,
} from "./readme-organization-data.ts";
import { type Ways } from "./readme-work.ts";
import { inOrder } from "./sequence.ts";

// The issues of the made-up workshop of the front page: each is reported by its person and then
// worked on, as the workshop would, through the API.

/** Everything the issues need: the people and the IDs of what the issues use. */
export interface Workshop {
  readonly people: People;
  readonly ways: Ways;
}

/** An ID by name, or a plain stop when the made-up organization names something unknown. */
function known(ids: Readonly<Record<string, string>>, name: string): string {
  const found = ids[name];
  if (found === undefined) {
    throw new TypeError(`The workshop has nothing called ${name}.`);
  }
  return found;
}

/** The statuses an issue of the repairs passes, one change at a time, from the first to `to`. */
export function wayTo(to: string): readonly string[] {
  const first = repairWorkflow.statuses[0]?.key ?? to;
  const before = new Map<string, string>();
  const waiting = [first];
  for (let at = waiting.shift(); at !== undefined; at = waiting.shift()) {
    for (const change of repairWorkflow.changes.filter((each) => each.from === at)) {
      if (!before.has(change.to) && change.to !== first) {
        before.set(change.to, at);
        waiting.push(change.to);
      }
    }
  }
  const way: string[] = [];
  for (let at: string | undefined = to; at !== undefined && at !== first; at = before.get(at)) {
    way.unshift(at);
  }
  return way;
}

/** Reports an issue as its reporter would: its type, title and description. */
async function report(shop: Workshop, job: Job, description?: string): Promise<MadeIssue> {
  const reporter = shop.people.sessions[job.reporter] ?? shop.people.admin;
  const made = dataOf(
    await reporter.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project: projects[job.place].shortName } },
      body: {
        issue_type: known(shop.ways.types, job.type),
        title: job.title,
        ...(description === undefined ? {} : { description }),
      },
    }),
  );
  if (typeof made.id !== "string" || typeof made.readable_id !== "string") {
    throw new TypeError("The server made an issue without an ID.");
  }
  return { id: made.id, key: made.readable_id };
}

/** The values of the own fields of an issue, by the IDs of the fields. */
function valuesOf(shop: Workshop, job: Job): Record<string, string> {
  return {
    ...(job.customer === undefined ? {} : { [shop.ways.fields.customer]: job.customer }),
    ...(job.frameSize === undefined
      ? {}
      : { [shop.ways.fields.frameSize]: known(shop.ways.frameSizes, job.frameSize) }),
  };
}

/** Gives an issue its priority, its person, its values, its status, its label and its tags. */
async function workOn(shop: Workshop, job: Job, issue: MadeIssue): Promise<void> {
  const { admin } = shop.people;
  const path = { params: { path: { issue: issue.key } } };
  const assigned = job.assigned === undefined ? [] : [idOf(shop.people, job.assigned)];
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    ...path,
    body: {
      version: await versionOf(admin, issue),
      priority: job.priority ?? "normal",
      assigned_to: assigned,
      fields: valuesOf(shop, job),
    },
  });
  const way =
    job.status === undefined ? [] : job.place === "repairs" ? wayTo(job.status) : [job.status];
  await inOrder(
    way.map((status) => async () => {
      await admin.client.PATCH("/api/v1/issues/{issue}", {
        ...path,
        // A status change that asks for who does the work is sent with the answer.
        body: { version: await versionOf(admin, issue), status, assigned_to: assigned },
      });
    }),
  );
  if (job.label !== undefined) {
    await admin.client.POST("/api/v1/issues/{issue}/labels", {
      ...path,
      body: { label: known(shop.ways.labels, job.label) },
    });
  }
  await inOrder(
    (job.tags ?? []).map((name) => async () => {
      await admin.client.POST("/api/v1/issues/{issue}/tags", { ...path, body: { name } });
    }),
  );
}

/** Reports an issue and works on it. */
export async function createJob(
  shop: Workshop,
  job: Job,
  description?: string,
): Promise<MadeIssue> {
  const issue = await report(shop, job, description);
  await workOn(shop, job, issue);
  return issue;
}

/** The cargo bike with its comments, its sub-issue and the order it waits for. */
export async function cargoBikeStory(shop: Workshop): Promise<void> {
  const { admin } = shop.people;
  const bike = await createJob(shop, cargoBike, cargoBike.description);
  const rotor = await createJob(shop, cargoBikeRotor);
  await admin.client.PUT("/api/v1/issues/{issue}/parent", {
    params: { path: { issue: rotor.key } },
    body: { parent: bike.key, version: await versionOf(admin, rotor) },
  });
  const hub = await createJob(shop, cargoBikeHub);
  const kinds = dataOf(await admin.client.GET("/api/v1/relation-kinds")).items;
  const blocks = kinds.find((kind) => kind.forward === "blocks")?.id;
  if (typeof blocks === "string") {
    await admin.client.POST("/api/v1/issues/{issue}/relations", {
      params: { path: { issue: hub.key } },
      body: { direction: "forward", other: bike.key, relation_kind: blocks },
    });
  }
  await inOrder(
    cargoBikeComments.map(({ who, text }) => async () => {
      await comment(shop.people.sessions[who] ?? admin, bike.key, text);
    }),
  );
}
