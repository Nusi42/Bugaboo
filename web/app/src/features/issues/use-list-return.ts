import { useLayout } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

import { rememberList } from "../../shared/list-return.ts";
import { issueAddress } from "../../shared/paths.ts";
import { panelClosedAddress, type SearchState } from "../../shared/search-address.ts";

/**
 * What a list of a project does for the issues it opens. It remembers its address, so *Back to
 * list* of an issue returns to the same search, sort, group and view; and where a phone is asked
 * for an issue in the side panel (an old address), it opens the issue's page instead, as a phone has
 * no room for a panel (UX §3.4). Gives whether that page is on its way, so the list draws nothing.
 */
export function useListReturn(shortName: string, state: SearchState): boolean {
  const open = state.open;
  const toPage = useLayout() === "phone" && open !== undefined;
  const navigate = useNavigate();
  // The address still says this list only while it is the page; as the issue's page opens, it does not.
  useEffect(() => {
    if (state.project === shortName) {
      rememberList(shortName, panelClosedAddress(state));
    }
  }, [shortName, state]);
  useEffect(() => {
    if (toPage && open !== undefined) {
      void navigate({ href: issueAddress(open), replace: true });
    }
  }, [toPage, open, navigate]);
  return toPage;
}
