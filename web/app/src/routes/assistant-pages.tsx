import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { mySettingsRoute } from "./layouts.tsx";

/** *My settings › Assistant*: the look-ups the assistant may use for one of the person's questions (WP-15). */
export const assistantSettingsRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.assistantSettings,
  component: lazyRouteComponent(() => import("../features/my-settings/index.ts"), "AssistantPage"),
});
