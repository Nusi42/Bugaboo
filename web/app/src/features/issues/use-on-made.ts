import { useNavigate } from "@tanstack/react-router";

import { doNothing } from "../../shared/do-nothing.ts";
import { queryClient } from "../../shared/query-client.ts";
import { formLayoutOptions } from "../layouts/index.ts";
import { closeOverlay } from "../shell/index.ts";
import { issueOptions } from "./issues.ts";
import { hrefOf } from "./issue-show.ts";
import { useOpenIssuesIn } from "./open-in.ts";

/**
 * Reads the new issue and the layout of its page before it opens, so the page is drawn whole at once:
 * not an empty page, then grey shapes for a moment, then the issue (UX §6). A read that fails opens
 * the page all the same, which says why.
 */
async function readAhead(key: string, project: string): Promise<void> {
  const detail = await queryClient.query(issueOptions(key));
  await queryClient.query(
    formLayoutOptions({ project, issueType: detail.issue.issue_type?.id, kind: "issue-page" }),
  );
}

/** What the new-issue window does with the issue it made; it is done once the issue is open. */
export type OnMade = (key: string, project: string) => Promise<void>;

/**
 * What happens when the new-issue window made an issue: the issue is read, then the window closes
 * and the issue opens the way the person chose to open issues (the focus view or the side panel).
 */
export function useOnMade(): OnMade {
  const openIn = useOpenIssuesIn();
  const navigate = useNavigate();
  return async (key, project) => {
    // A read that fails opens the page all the same; the page says why.
    await readAhead(key, project).catch(doNothing);
    closeOverlay();
    void navigate({ href: hrefOf(openIn, undefined, project, key) });
  };
}
