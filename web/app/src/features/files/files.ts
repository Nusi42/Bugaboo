import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";

/** The files of an issue and of the comments under it, with the signed links of the reader. */
export function useIssueFiles(issue: string) {
  return api.query.useQuery("get", "/api/v1/issues/{issue}/files", {
    params: { path: { issue } },
  });
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
