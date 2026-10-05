import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { guestRoute } from "./layouts.tsx";

export const signUpRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.signUp,
  component: lazyRouteComponent(() => import("../features/sign-up/index.ts"), "SignUpPage"),
});

export const invitationRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.invitation,
  component: lazyRouteComponent(() => import("../features/invitation/index.ts"), "InvitationPage"),
});
