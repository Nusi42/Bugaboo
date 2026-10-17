import type { Schema } from "@bugaboo/api-client";
import { useEffect, useRef } from "react";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";
import { refreshHistory } from "../../shared/query-keys.ts";
import type { FileItem } from "./file-item.ts";

/** How long the list of files waits before it is read again while a preview is on its way, in milliseconds. */
export const PREVIEW_POLL_MS = 1500;

/**
 * How many times in a row the list is read again for a preview that is not there yet, about a minute
 * with [`PREVIEW_POLL_MS`]. After that the page stops asking: a preview that takes longer than that
 * has a problem the page cannot cure. Reloading the page, or coming back to it, starts again.
 */
export const PREVIEW_POLL_ROUNDS_MAX = 40;

/** Whether the preview of any of these files is still being made. */
export function hasPreviewComing(items: readonly FileItem[]): boolean {
  return items.some((item) => item.preview === "pending");
}

/**
 * How long to wait before the list is read again: the time of [`PREVIEW_POLL_MS`] while a preview is
 * on its way and fewer than [`PREVIEW_POLL_ROUNDS_MAX`] rounds were spent on it in a row, and
 * `false` (no more rounds) otherwise.
 */
export function previewPollDelay(items: readonly FileItem[], round: number): number | false {
  return hasPreviewComing(items) && round < PREVIEW_POLL_ROUNDS_MAX ? PREVIEW_POLL_MS : false;
}

/**
 * The files of an issue and of the comments under it, with the signed links of the reader. While
 * the preview of a file is being made the list is read again every moment, so the picture shows
 * as soon as it is there, without the person reloading the page; the rounds are bounded, and a
 * return to the page starts them again.
 */
export function useIssueFiles(issue: string) {
  const rounds = useRef(0);
  useEffect(() => {
    const again = (): void => {
      rounds.current = 0;
    };
    window.addEventListener("focus", again);
    return () => {
      window.removeEventListener("focus", again);
    };
  }, []);
  return api.query.useQuery(
    "get",
    "/api/v1/issues/{issue}/files",
    { params: { path: { issue } } },
    {
      refetchInterval: (
        query: Readonly<{ state: Readonly<{ data: Schema<"Page_AttachmentItem"> | undefined }> }>,
      ) => {
        const items = query.state.data?.items ?? [];
        rounds.current = hasPreviewComing(items) ? rounds.current + 1 : 0;
        return previewPollDelay(items, rounds.current);
      },
    },
  );
}

/** The most bytes one file may have in the project of an issue; 0 until the rules are read. */
export function useFileLimit(issue: string): number {
  return useFileRules(issue).data?.limit_bytes ?? 0;
}

/** What the person may add to an issue, and the limits of its project. */
export function useFileRules(issue: string) {
  return api.query.useQuery("get", "/api/v1/issues/{issue}/file-rules", {
    params: { path: { issue } },
  });
}

/**
 * The list of files of an issue, read again: a file was added, deleted or brought back. The
 * history has a row for it (M4-F50), so it is read again too.
 */
export function refreshFiles(): Promise<void> {
  refreshHistory();
  return queryClient.invalidateQueries({
    queryKey: ["get", "/api/v1/issues/{issue}/files"],
  });
}
