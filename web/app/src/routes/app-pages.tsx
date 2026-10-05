import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { HomePage } from "../features/home/index.ts";
import { paths } from "../shared/paths.ts";
import { adminRoute, appRoute } from "./layouts.tsx";

// The pages that are not the first one are loaded when they are first opened, so the first page
// stays small (coding-standards section 6).

export const homeRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.home,
  component: HomePage,
});

export const securityRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.security,
  component: lazyRouteComponent(() => import("../features/account/index.ts"), "SecurityPage"),
});

export const sessionsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.sessions,
  component: lazyRouteComponent(() => import("../features/account/index.ts"), "SessionsPage"),
});

export const invitationsRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: paths.invitations,
  component: lazyRouteComponent(() => import("../features/admin/index.ts"), "InvitationsPage"),
});

export const statusRoute = createRoute({
  getParentRoute: () => adminRoute,
  path: paths.status,
  component: lazyRouteComponent(() => import("../features/admin/index.ts"), "StatusPage"),
});
