import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { mySettingsRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

// The theme editor is a chunk of its own beside the lists of themes (PERF-04): the feature loads it.
const ThemeEditorScreen = lazyRouteComponent(
  async () => (await import("../features/theme-editor/index.ts")).loadThemeEditor(),
  "ThemeEditorPage",
);

/** The theme editor of a theme for everyone, `/settings/themes/<ID>` (UX §5.6). */
export const themeRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.theme,
  loader: loadsScreen(ThemeEditorScreen),
  component: function Theme() {
    const id = useAddressPart(routeParam.theme);
    return (
      <OnlyWith path={paths.theme}>
        {id === undefined ? null : <ThemeEditorScreen theme={id} />}
      </OnlyWith>
    );
  },
});

/** The theme editor of a personal theme, `/me/appearance/themes/<ID>` (UX §5.6). */
export const myThemeRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.myTheme,
  loader: loadsScreen(ThemeEditorScreen),
  component: function MyTheme() {
    const id = useAddressPart(routeParam.theme);
    return id === undefined ? null : <ThemeEditorScreen theme={id} />;
  },
});
