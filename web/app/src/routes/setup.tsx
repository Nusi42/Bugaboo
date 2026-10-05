import { createRoute, lazyRouteComponent, redirect } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { guestRoute } from "./layouts.tsx";
import { loadStatus } from "./start.ts";

/**
 * The first start in the browser. It only exists while no administrator does; afterwards the
 * address leads to the sign-in page.
 */
export const setupRoute = createRoute({
  getParentRoute: () => guestRoute,
  path: paths.setup,
  beforeLoad: async (): Promise<void> => {
    if (!(await loadStatus()).setup_needed) {
      throw redirect({ to: paths.signIn });
    }
  },
  component: lazyRouteComponent(() => import("../features/setup/index.ts"), "SetupPage"),
});
