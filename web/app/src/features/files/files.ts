import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";
import type { FileItem } from "./file-item.ts";

/** How long the list of files waits before it is read again while a preview is on its way, in milliseconds. */
export const PREVIEW_POLL_MS = 1500;

/** Whether the preview of any of these files is still being made. */
export function hasPreviewComing(items: readonly FileItem[]): boolean {
  return items.some((item) => item.preview === "pending");
}

/**
 * The files of an issue and of the comments under it, with the signed links of the reader. While
 * the preview of a file is being made the list is read again every moment, so the picture shows
 * as soon as it is there, without the person reloading the page.
 */
export function useIssueFiles(issue: string) {
  return api.query.useQuery(
    "get",
    "/api/v1/issues/{issue}/files",
    { params: { path: { issue } } },
    {
      refetchInterval: (
        query: Readonly<{ state: Readonly<{ data: Schema<"Page_AttachmentItem"> | undefined }> }>,
      ) => (hasPreviewComing(query.state.data?.items ?? []) ? PREVIEW_POLL_MS : false),
    },
  );
}

/** What the person may add to an issue, and the limits of its project. */
export function useFileRules(issue: string) {
  return api.query.useQuery("get", "/api/v1/issues/{issue}/file-rules", {
    params: { path: { issue } },
  });
}

/** The list of files of an issue, read again: a file was added, deleted or brought back. */
export function refreshFiles(): Promise<void> {
  return queryClient.invalidateQueries({
    queryKey: ["get", "/api/v1/issues/{issue}/files"],
  });
}
