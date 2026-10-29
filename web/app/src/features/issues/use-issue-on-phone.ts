import { useLayout } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { issueAddress } from "../../shared/paths.ts";
import type { SearchState } from "../../shared/search-address.ts";

/**
 * Where a phone is asked for an issue in the side panel (an old address) the list opens the
 * issue's page instead, as a phone has no room for a panel (UX §3.4). Gives whether that page is on its way, so the list draws nothing.
 */
export function useIssueOnPhone(state: SearchState): boolean {
  const open = state.open;
  const toPage = useLayout() === "phone" && open !== undefined;
  const navigate = useNavigate();
  useEffect(() => {
    if (toPage && open !== undefined) {
      void navigate({ href: issueAddress(open), replace: true });
    }
  }, [toPage, open, navigate]);
  return toPage;
}
