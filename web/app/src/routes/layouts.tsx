import { createRootRoute, createRoute, redirect } from "@tanstack/react-router";

import {
  AppLayout,
  ErrorPage,
  GuestLayout,
  NotFoundPage,
  PendingPage,
  SettingsFrame,
} from "../features/shell/index.ts";
import { paths } from "../shared/paths.ts";
import { currentPerson, type Person } from "../shared/session.ts";
import { OnlyWith } from "./OnlyWith.tsx";
import { Root } from "./Root.tsx";
import { loadStatus, startPath } from "./start.ts";

/** Whether the address says why the person is here: the sign-in page explains it. */
function hasReason(search: unknown): boolean {
  return typeof search === "object" && search !== null && "reason" in search;
}

export const rootRoute = createRootRoute({
  component: Root,
  notFoundComponent: NotFoundPage,
  errorComponent: ErrorPage,
  pendingComponent: PendingPage,
});

/** Pages that nobody needs to be signed in for. Somebody who is signed in goes to the start page. */
export const guestRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "guest",
  beforeLoad: async ({
    location,
  }: {
    readonly location: { readonly search: unknown };
  }): Promise<void> => {
    await loadStatus();
    // A sign-in that does not fit the page (M1-F51) still has its cookie, but the person is sent
    // here to sign in again, so the page with a reason is not left for the start page.
    if (!hasReason(location.search) && (await currentPerson()) !== null) {
      throw redirect({ to: paths.home });
    }
  },
  component: GuestLayout,
});

/** Whether the projects have joined the sidebar and quick search. */
let projectsJoined = false;

/**
 * The projects join the sidebar and quick search once somebody is signed in, so the pages before
 * sign-in do not carry them. Only the first time waits: a page that waited on every step would
 * show the next page a moment later, and the focus would have moved to the old one by then.
 */
async function joinProjects(): Promise<void> {
  if (projectsJoined) {
    return;
  }
  const projects = await import("../features/projects/index.ts");
  // Marked only now: when the import fails, the next page tries again. Two pages that waited
  // together join once.
  if (!projectsJoined) {
    projectsJoined = true;
    projects.registerProjects();
  }
}

/**
 * Pages of somebody who is signed in. Everybody else goes to sign in, or to the setup; somebody
 * who has to set up a second step first goes to that screen and can open nothing else.
 */
export const appRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "app",
  beforeLoad: async (): Promise<{ readonly person: Person }> => {
    const person = await currentPerson();
    if (person === null) {
      throw redirect({ to: await startPath() });
    }
    if (person.second_step_setup_required) {
      throw redirect({ to: paths.setUpSecondStep });
    }
    await joinProjects();
    return { person };
  },
  component: AppLayout,
});

/**
 * The page of somebody who is signed in and has to set up a second step: the same frame as the
 * pages before sign-in, with nothing of the app around it. Anybody else is taken where they belong.
 */
export const setUpRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "set-up",
  beforeLoad: async (): Promise<void> => {
    const person = await currentPerson();
    if (person === null) {
      throw redirect({ to: await startPath() });
    }
    if (!person.second_step_setup_required) {
      throw redirect({ to: paths.home });
    }
  },
  component: GuestLayout,
});

/** The frame of *My settings*: the menu of its topics next to the topic that is open. */
export const mySettingsRoute = createRoute({
  getParentRoute: () => appRoute,
  id: "my-settings",
  component: function MySettings() {
    return <SettingsFrame parent={paths.mySettings} />;
  },
});

/** The frame of *Organization settings*: the same, for the topics people hold a permission for. */
export const organizationSettingsRoute = createRoute({
  getParentRoute: () => appRoute,
  id: "organization-settings",
  component: function OrganizationSettings() {
    return (
      <OnlyWith path={paths.organizationSettings}>
        <SettingsFrame parent={paths.organizationSettings} />
      </OnlyWith>
    );
  },
});
