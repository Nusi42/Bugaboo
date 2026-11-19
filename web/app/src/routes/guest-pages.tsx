import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { guestRoute } from "./layouts.tsx";

export const signUpRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.signUp,
  component: lazyRouteComponent(() => import("../features/sign-up/index.ts"), "SignUpPage"),
});

/** The page the link to a new e-mail address opens (PRIV-06). */
export const eMailChangeRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.eMailChange,
  component: lazyRouteComponent(
    () => import("../features/my-settings/index.ts"),
    "EmailChangePage",
  ),
});

export const invitationRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.invitation,
  component: lazyRouteComponent(() => import("../features/invitation/index.ts"), "InvitationPage"),
});
