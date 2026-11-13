import { useNavigate } from "@tanstack/react-router";

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

/**
 * What happens when the new-issue window made an issue: the issue is read, then the window closes
 * and the issue opens the way the person chose to open issues (the focus view or the side panel).
 */
export function useOnMade(): (key: string, project: string) => void {
  const openIn = useOpenIssuesIn();
  const navigate = useNavigate();
  return (key, project) => {
    const open = (): void => {
      closeOverlay();
      void navigate({ href: hrefOf(openIn, undefined, project, key) });
    };
    readAhead(key, project).then(open, open);
  };
}
