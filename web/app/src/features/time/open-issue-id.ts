import { issueAddress } from "../../shared/paths.ts";

/** The Issue ID of the issue the page shows at this address, if it is the page of an issue. */
export function openIssueId(pathname: string): string | undefined {
  const prefix = issueAddress("");
  if (!pathname.startsWith(prefix)) {
    return undefined;
  }
  const id = pathname.slice(prefix.length).split("/")[0];
  return id === undefined || id === "" ? undefined : decodeURIComponent(id);
}
