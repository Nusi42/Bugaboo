import { useNavigate } from "@tanstack/react-router";

import { closeOverlay } from "../shell/index.ts";
import { hrefOf } from "./issue-show.ts";
import { useOpenIssuesIn } from "./open-in.ts";

/**
 * What happens when the new-issue window made an issue: it closes, and the issue opens the way
 * the person chose to open issues (the focus view or the side panel).
 */
export function useOnMade(): (key: string, project: string) => void {
  const openIn = useOpenIssuesIn();
  const navigate = useNavigate();
  return (key, project) => {
    closeOverlay();
    void navigate({ href: hrefOf(openIn, undefined, project, key) });
  };
}
