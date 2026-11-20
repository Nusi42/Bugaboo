import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { mySettingsRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

// The screens of privacy (PRIV-03, PRIV-05, ADR-0044, ADR-0045): loaded when first opened, like the other settings.

const MyDataScreen = lazyRouteComponent(
  () => import("../features/my-settings/index.ts"),
  "MyDataPage",
);
const KeepRulesScreen = lazyRouteComponent(
  () => import("../features/settings/index.ts"),
  "KeepRulesPage",
);
const DataExportsScreen = lazyRouteComponent(
  () => import("../features/settings/index.ts"),
  "DataExportsPage",
);

/** *My settings › My data*: every person's own. */
export const myDataRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.myData,
  component: MyDataScreen,
});

/** *Organization settings › Privacy › Keep and delete rules*. */
export const keepRulesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.keepRules,
  loader: loadsScreen(KeepRulesScreen),
  component: function KeepRules() {
    return (
      <OnlyWith path={paths.keepRules}>
        <KeepRulesScreen />
      </OnlyWith>
    );
  },
});

/** *Organization settings › Privacy › Data exports*. */
export const dataExportsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.dataExports,
  loader: loadsScreen(DataExportsScreen),
  component: function DataExports() {
    return (
      <OnlyWith path={paths.dataExports}>
        <DataExportsScreen />
      </OnlyWith>
    );
  },
});
