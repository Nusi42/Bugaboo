import { dataOf, type Schema, workflowEditPart } from "@bugaboo/api-client";
import { type Page } from "@playwright/test";

import { roleName, signedInAdmin, createPeopleWithRoles } from "./access.ts";
import { cleo } from "./accounts.ts";
import { type ApiSession } from "./api.ts";
import { createOwnField } from "./fields.ts";
import {
  firstIssueType,
  issueOf,
  prepareWeb,
  tickBox,
  setStatusAndPriority,
  versionOf,
  type MadeIssue,
} from "./issues.ts";
import { inOrder } from "./sequence.ts";
import { statusOf } from "./many-changes.ts";
import { type Instance } from "./server.ts";

/** The own field that the workflow tests ask for and require. */
export const foundInVersion = "Found in version";

/** The key of the status that the tests add to *Simple*. */
export const testing = "testing";

/** The workflow as the editor sends it back: what *Publish* replaces. */
export type Content = Pick<Schema<"WorkflowEditorBody">, "name" | "statuses" | "changes">;

/** A copy of rules that the request may take (a body that is sent is not read-only). */
function rulesBody(rules: Schema<"StatusRulesBody">) {
  return {
    who: {
      roles: [...(rules.who?.roles ?? [])],
      groups: [...(rules.who?.groups ?? [])],
      assigned: rules.who?.assigned ?? false,
      reported_by: rules.who?.reported_by ?? false,
    },
    only_if: (rules.only_if ?? []).map((each) => ({ field: each.field, test: each.test })),
    must_fill: [...(rules.must_fill ?? [])],
    ask_for: [...(rules.ask_for ?? [])],
  };
}

/**
 * Gets a server ready for the tests of workflows: the project WEB with its people (Ana the Viewer,
 * Bob the Team member, Lea the Guest) and Cleo, a Contributor, and the own field *Found in
 * version*. A new organization has *Simple* for Task, *Bug* for Bug and *Development* for Feature.
 */
export async function prepareWorkflows(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  await createOwnField(admin, foundInVersion, "text");
  await createPeopleWithRoles(server, admin, [
    { person: cleo, role: roleName.contributor, project: "WEB" },
  ]);
  await admin.dispose();
}

/** The workflow of the whole organization for all issue types (*Simple*), as the editor shows it. */
async function simple(admin: ApiSession) {
  return dataOf(await admin.client.GET("/api/v1/workflows/editor", { params: { query: {} } }));
}

/** The editor of a workflow, as the server answers it. */
type Editor = Awaited<ReturnType<typeof simple>>;

/**
 * Publishes what `change` made of the statuses and status changes of the workflow in the editor,
 * the way the editor does: takes the edit lock, publishes, and lets the lock go.
 */
async function publishChanged(
  admin: ApiSession,
  editorOf: () => Promise<Editor>,
  change: (content: Content) => Content,
): Promise<void> {
  const editor = await editorOf();
  const slot = editor.workflow;
  if (slot === undefined || slot === null) {
    throw new TypeError("The place has no workflow of its own.");
  }
  const what = { record: { kind: "workflow", id: slot.id }, part: workflowEditPart } as const;
  const taken = dataOf(
    await admin.client.POST("/api/v1/edit-locks", {
      body: { target: what, editor_id: crypto.randomUUID() },
    }),
  );
  if (taken.status !== "taken") {
    throw new TypeError("Somebody else is editing the workflow.");
  }
  const changed = change(editor);
  dataOf(
    await admin.client.PUT("/api/v1/workflows/{workflow}", {
      params: { path: { workflow: slot.id } },
      body: {
        version: slot.version,
        lock_token: taken.lock_token,
        name: changed.name,
        statuses: changed.statuses.map((each) => ({
          key: each.key,
          name: each.name,
          status_group: each.status_group,
        })),
        changes: changed.changes.map((each) => ({
          from: each.from,
          to: each.to,
          button: each.button,
          rules: rulesBody(each.rules),
        })),
      },
    }),
  );
  await admin.client.POST("/api/v1/edit-locks/release", {
    body: { target: what, lock_token: taken.lock_token },
  });
}

/**
 * Changes *Simple* the way the editor does: takes the edit lock, publishes what `change` made of
 * the statuses and status changes, and lets the lock go.
 */
export async function changeSimple(
  admin: ApiSession,
  change: (content: Content) => Content,
): Promise<void> {
  await publishChanged(admin, () => simple(admin), change);
}

/**
 * Gives a project a workflow of its own, a copy of the one that counts there, in which the status
 * change from *Open* to *In progress* asks for who does the work (*Ask for*, WF-04).
 */
export async function askForWhoWorks(admin: ApiSession, project: string): Promise<void> {
  const editorOf = async (): Promise<Editor> =>
    dataOf(await admin.client.POST("/api/v1/workflows", { body: { project } }));
  await publishChanged(admin, editorOf, (content) => ({
    ...content,
    changes: content.changes.map((each) =>
      each.from === "open" && each.to === "in-progress"
        ? { ...each, rules: { ...each.rules, ask_for: ["assigned-to"] } }
        : each,
    ),
  }));
}

/** The content with the status change from one status to another made for these roles only (*Who can do it*). */
export function onlyForRoles(
  content: Content,
  change: { readonly from: string; readonly to: string },
  roles: readonly string[],
): Content {
  return {
    ...content,
    changes: content.changes.map((each) =>
      each.from === change.from && each.to === change.to
        ? { ...each, rules: { ...each.rules, who: { ...each.rules.who, roles: [...roles] } } }
        : each,
    ),
  };
}

/** The content with the status change from one status to another that needs these fields filled in first. */
export function mustFill(
  content: Content,
  change: { readonly from: string; readonly to: string },
  fields: readonly string[],
): Content {
  return {
    ...content,
    changes: content.changes.map((each) =>
      each.from === change.from && each.to === change.to
        ? { ...each, rules: { ...each.rules, must_fill: [...fields] } }
        : each,
    ),
  };
}

/** Creates an issue of the first issue type (*Bug*) in the project WEB. */
export async function createBug(admin: ApiSession, title: string): Promise<MadeIssue> {
  const made = dataOf(
    await admin.client.POST("/api/v1/projects/{project}/issues", {
      params: { path: { project: "WEB" } },
      body: { issue_type: await firstIssueType(admin), title },
    }),
  );
  if (
    made.id === undefined ||
    made.id === null ||
    made.readable_id === undefined ||
    made.readable_id === null
  ) {
    throw new TypeError("The server made an issue without an ID.");
  }
  return { id: made.id, key: made.readable_id };
}

/** Puts the issues in the status *In progress*, one after the other. */
export async function startWorkingOn(
  admin: ApiSession,
  issues: readonly MadeIssue[],
): Promise<void> {
  await inOrder(
    issues.map((issue) => async () => {
      await setStatusAndPriority(admin, issue, await versionOf(admin, issue), {
        status: "in-progress",
        priority: "normal",
      });
    }),
  );
}

/** The issues the tests of changing many issues start from: two Bugs and a Task, all *In progress*. */
export async function threeInProgress(admin: ApiSession): Promise<readonly MadeIssue[]> {
  const bugs = [await createBug(admin, "First bug"), await createBug(admin, "Second bug")];
  const task = await issueOf(admin, "WEB-1");
  await startWorkingOn(admin, [...bugs, task]);
  return [...bugs, task];
}

/** The status each of the issues has now, in the order of the issues. */
export async function statusesOf(
  admin: ApiSession,
  issues: readonly MadeIssue[],
): Promise<(string | undefined)[]> {
  const found: (string | undefined)[] = [];
  await inOrder(
    issues.map((issue) => async () => {
      found.push(await statusOf(admin, issue));
    }),
  );
  return found;
}

/** Ticks the issues of the list one after the other. */
export async function tickAll(page: Page, issues: readonly MadeIssue[]): Promise<void> {
  await inOrder(
    issues.map((issue) => async () => {
      await tickBox(page, issue.key).check({ force: true });
    }),
  );
}
