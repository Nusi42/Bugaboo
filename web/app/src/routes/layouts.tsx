import { createRootRoute, createRoute, lazyRouteComponent, redirect } from "@tanstack/react-router";

import {
  AppLayout,
  ErrorPage,
  GuestLayout,
  UnknownAddressPage,
  PendingPage,
} from "../features/shell/index.ts";
import { paths } from "../shared/paths.ts";
import { rememberReturn } from "../shared/return-to.ts";
import { currentPerson, type Person } from "../shared/session.ts";
import { OnlyWith } from "./OnlyWith.tsx";
import { joinTheShell } from "./joined.ts";
import { loadsScreen } from "./preload.ts";
import { Root } from "./Root.tsx";
import { loadStatus, startPath } from "./start.ts";

/** Whether the address says why the person is here: the sign-in page explains it. */
function hasReason(search: unknown): boolean {
  return typeof search === "object" && search !== null && "reason" in search;
}

export const rootRoute = createRootRoute({
  component: Root,
  notFoundComponent: UnknownAddressPage,
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
    readonly location: { readonly search: unknown; readonly pathname: string };
  }): Promise<void> => {
    await loadStatus();
    // The link to a new e-mail address works with a session too: its secret proves the address.
    if (location.pathname === paths.eMailChange) {
      return;
    }
    // A sign-in that does not fit the page (M1-F51) still has its cookie, but the person is sent
    // here to sign in again, so the page with a reason is not left for the start page.
    if (!hasReason(location.search) && (await currentPerson()) !== null) {
      throw redirect({ to: paths.home });
    }
  },
  component: GuestLayout,
});

/**
 * Pages of somebody who is signed in. Everybody else goes to sign in, or to the setup; somebody
 * who has to set up a second step first goes to that screen and can open nothing else.
 */
export const appRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "app",
  beforeLoad: async ({
    location,
  }: {
    readonly location: { readonly href: string };
  }): Promise<{ readonly person: Person }> => {
    const person = await currentPerson();
    if (person === null) {
      // The page asked for is kept when it is the consent step of an AI app: after signing in the
      // person comes back to it.
      rememberReturn(location.href);
      throw redirect({ to: await startPath() });
    }
    if (person.second_step_setup_required) {
      throw redirect({ to: paths.setUpSecondStep });
    }
    // The person's own settings come before the first page, so the look is theirs from the start.
    await joinTheShell();
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

/** The frame of the settings screens, in the feature of the settings: not on the first page (PERF-04). */
const SettingsFrame = lazyRouteComponent(
  () => import("../features/settings/index.ts"),
  "SettingsFrame",
);

/** The frame of *My settings*: the menu of its topics next to the topic that is open. */
export const mySettingsRoute = createRoute({
  getParentRoute: () => appRoute,
  id: "my-settings",
  loader: loadsScreen(SettingsFrame),
  component: function MySettings() {
    return <SettingsFrame parent={paths.mySettings} />;
  },
});

/** The frame of *Organization settings*: the same, for the topics people hold a permission for. */
export const organizationSettingsRoute = createRoute({
  getParentRoute: () => appRoute,
  id: "organization-settings",
  loader: loadsScreen(SettingsFrame),
  component: function OrganizationSettings() {
    return (
      <OnlyWith path={paths.organizationSettings}>
        <SettingsFrame parent={paths.organizationSettings} />
      </OnlyWith>
    );
  },
});
