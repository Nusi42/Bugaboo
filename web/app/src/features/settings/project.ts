import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { isMySettings, type MySettings } from "../../shared/my-settings.ts";
import { type OptimisticMutation, useOptimisticMutation } from "../../shared/use-optimistic.ts";
import { after, type Edit } from "./organization.ts";

/** The settings that can be set on one project, with the values that count there. */
export function projectOptions(projectId: string) {
  return api.query.queryOptions("get", "/api/v1/projects/{project}/settings", {
    params: { path: { project: projectId } },
  });
}

/** Sends an edit of a project's setting to the server; a project has no lock of its own. */
async function send(projectId: string, edit: Edit): Promise<void> {
  const path = { params: { path: { project: projectId, key: edit.key } } };
  if (edit.kind === "change") {
    await api.client.PUT("/api/v1/projects/{project}/settings/{key}", {
      ...path,
      body: { value: edit.value },
    });
  } else if (edit.kind === "reset") {
    await api.client.DELETE("/api/v1/projects/{project}/settings/{key}", path);
  }
}

/** The settings of a project and the edits that show at once (UX-06). */
export function useProjectSettings(projectId: string): {
  readonly settings: MySettings | undefined;
  readonly failed: unknown;
  readonly edits: OptimisticMutation<Edit>;
} {
  const query = useQuery(projectOptions(projectId));
  const edits = useOptimisticMutation({
    queryKey: projectOptions(projectId).queryKey,
    isCached: isMySettings,
    apply: after,
    send: (edit: Edit) => send(projectId, edit),
  });
  return { settings: query.data, failed: query.error, edits };
}
