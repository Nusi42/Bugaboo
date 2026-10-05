import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";

import { paths } from "../../shared/paths.ts";
import { useSignOut } from "../../shared/use-sign-out.ts";
import { useMe } from "../../shared/use-me.ts";
import { useShortcutHost, useShortcuts } from "../../shared/use-shortcuts.ts";
import { closeOverlay, openOverlay } from "./overlay-state.ts";
import { mayOpen } from "./places.ts";
import type { SidebarProps } from "./Sidebar.tsx";

/** What the sidebar, the menu of a tablet and the sheet of a phone need to know about the person. */
export type PersonProps = Pick<
  SidebarProps,
  "organizationSettings" | "name" | "onSignOut" | "signingOut"
>;

/**
 * What the frame of the signed-in app does: it applies the person's appearance, listens for the
 * shortcuts that work everywhere, moves the focus to a new page, and knows how to sign out.
 */
export function useShell(): {
  readonly person: PersonProps;
  readonly signOutProblem: unknown;
} {
  const me = useMe();
  const navigate = useNavigate();
  const signOut = useSignOut();
  useShortcutHost();
  // A layer that was open does not stay open on the next page, or for the next person.
  const router = useRouter();
  useEffect(() => {
    const stop = router.subscribe("onResolved", closeOverlay);
    return () => {
      stop();
      closeOverlay();
    };
  }, [router]);
  useShortcuts({
    "quick-search": () => {
      openOverlay("search");
    },
    "show-shortcuts": () => {
      openOverlay("shortcuts");
    },
    "go-home": () => {
      void navigate({ to: paths.home });
    },
  });
  return {
    person: {
      organizationSettings: mayOpen(paths.organizationSettings, me?.permissions ?? []),
      name: me?.display_name,
      onSignOut: signOut.signOut,
      signingOut: signOut.isPending,
    },
    signOutProblem: signOut.error,
  };
}
