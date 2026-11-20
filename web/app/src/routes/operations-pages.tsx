import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { mySettingsRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

const BackupsScreen = lazyRouteComponent(() => import("../features/admin/index.ts"), "BackupsPage");
const AboutScreen = lazyRouteComponent(() => import("../features/admin/index.ts"), "AboutPage");

export const backupsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.backups,
  loader: loadsScreen(BackupsScreen),
  component: function Backups() {
    return (
      <OnlyWith path={paths.backups}>
        <BackupsScreen />
      </OnlyWith>
    );
  },
});

/** *About*: every signed-in person may open it, so it asks for no permission. */
export const aboutRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.about,
  loader: loadsScreen(AboutScreen),
  component: AboutScreen,
});
