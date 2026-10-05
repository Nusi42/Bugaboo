import { createRootRoute, createRoute, redirect } from "@tanstack/react-router";

import {
  AppLayout,
  ErrorPage,
  GuestLayout,
  NotFoundPage,
  PendingPage,
} from "../features/shell/index.ts";
import { paths } from "../shared/paths.ts";
import { currentPerson, type Person } from "../shared/session.ts";
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

/** Pages of somebody who is signed in. Everybody else goes to sign in, or to the setup. */
export const appRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "app",
  beforeLoad: async (): Promise<{ readonly person: Person }> => {
    const person = await currentPerson();
    if (person === null) {
      throw redirect({ to: await startPath() });
    }
    return { person };
  },
  component: AppLayout,
});

/** Pages for administrators. Everybody else goes to the start page. */
export const adminRoute = createRoute({
  getParentRoute: () => appRoute,
  id: "admin",
  beforeLoad: ({ context }: { readonly context: { readonly person: Person } }): void => {
    if (!context.person.administrator) {
      throw redirect({ to: paths.home });
    }
  },
});
