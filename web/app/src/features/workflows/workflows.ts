import { dataOf, workflowEditPart, type EditWhat, type Schema } from "@bugaboo/api-client";
import { useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { refreshWorkflows } from "../../shared/query-keys.ts";
import { bodyOf, type Draft } from "./draft-model.ts";

/** The workflow of a place as the editor shows it. */
export type WorkflowEditor = Schema<"WorkflowEditorBody">;
/** What *Publish* would drop, and the issues in it. */
export type PublishPreview = Schema<"PublishPreviewAnswer">;
/** What *Try it* found. */
export type Tried = Schema<"TriedBody">;

/** Which workflow to edit: a project (none for the whole organization) and an issue type (none for all). */
export interface EditorQuery {
  readonly project: string | undefined;
  readonly issueType: string | undefined;
}

/** The workflow that counts at a place, for somebody who may change it. */
export function useWorkflowEditor({ project, issueType }: EditorQuery) {
  return useQuery({
    ...api.query.queryOptions("get", "/api/v1/workflows/editor", {
      params: {
        query: {
          ...(project === undefined ? {} : { project }),
          ...(issueType === undefined ? {} : { issue_type: issueType }),
        },
      },
    }),
    select: (body: WorkflowEditor) => body,
  });
}

/** *Customize*: makes the workflow of the place as a copy of the one that counts there. */
export function useCreateWorkflow({ project, issueType }: EditorQuery) {
  return useMutation({
    mutationFn: async () =>
      dataOf(
        await api.client.POST("/api/v1/workflows", {
          body: {
            ...(project === undefined ? {} : { project }),
            ...(issueType === undefined ? {} : { issue_type: issueType }),
          },
        }),
      ),
    onSuccess: refreshWorkflows,
  });
}

/** What *Publish* is sent: the draft, the version it started from, the lock, and where issues go. */
export interface PublishInput {
  readonly workflow: string;
  readonly version: number;
  readonly lockToken: string;
  readonly draft: Draft;
  /** For each removed status that issues are in: the status they go to. */
  readonly moves: readonly { readonly from: string; readonly to: string }[];
}

/** Publishes a workflow under the edit lock the person holds. */
export function usePublishWorkflow() {
  return useMutation({
    mutationFn: async ({ workflow, version, lockToken, draft, moves }: PublishInput) =>
      dataOf(
        await api.client.PUT("/api/v1/workflows/{workflow}", {
          params: { path: { workflow } },
          body: {
            version,
            lock_token: lockToken,
            name: draft.name,
            ...bodyOf(draft),
            moves: moves.map((each) => ({ from: each.from, to: each.to })),
          },
        }),
      ),
    onSettled: refreshWorkflows,
  });
}

/** Asks which statuses publishing the draft would drop and which issues are in them. */
export function usePublishPreview() {
  return useMutation({
    mutationFn: async ({ workflow, draft }: Pick<PublishInput, "workflow" | "draft">) =>
      dataOf(
        await api.client.POST("/api/v1/workflows/{workflow}/publish-preview", {
          params: { path: { workflow } },
          body: bodyOf(draft),
        }),
      ),
  });
}

/** Takes the workflow of a place away: the place follows its parent again. */
export function useDeleteWorkflow() {
  return useMutation({
    mutationFn: async (input: { readonly workflow: string; readonly version: number }) =>
      dataOf(
        await api.client.DELETE("/api/v1/workflows/{workflow}", {
          params: { path: { workflow: input.workflow }, query: { version: input.version } },
        }),
      ),
    onSuccess: refreshWorkflows,
  });
}

/** *Try it*: the status changes a person would get on an issue, and why not the others. */
export function useTryIt() {
  return useMutation({
    mutationFn: async (input: { readonly issue: string; readonly person: string }) =>
      dataOf(
        await api.client.GET("/api/v1/issues/{issue}/status-changes", {
          params: { path: { issue: input.issue }, query: { person: input.person } },
        }),
      ),
  });
}

/** What the lock on a workflow is about: the workflow, edited as a whole. */
export function whatOf(workflow: string): EditWhat {
  return { record: { kind: "workflow", id: workflow }, part: workflowEditPart };
}
