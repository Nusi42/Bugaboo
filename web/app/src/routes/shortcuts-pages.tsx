import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { shortcutIdOf, type ShortcutId } from "../shared/shortcuts.ts";
import { mySettingsRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";

// The page of the shortcuts and its editor are loaded when it is first opened, so the first page stays small
// (coding-standards section 6).
const ShortcutsScreen = lazyRouteComponent(
  () => import("../features/my-settings/index.ts"),
  "ShortcutsPage",
);

/** The page of the keyboard shortcuts, with the address saying which one is open. */
export const shortcutsRoute = createRoute({
  getParentRoute: () => mySettingsRoute,
  path: paths.shortcuts,
  validateSearch: (
    search: Readonly<Record<string, unknown>>,
  ): { edit?: ShortcutId | undefined } => ({ edit: shortcutIdOf(search["edit"]) }),
  loader: loadsScreen(ShortcutsScreen),
  component: function Shortcuts() {
    const { edit } = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <ShortcutsScreen
        edit={edit}
        onEdit={(id) => {
          void navigate({ to: paths.shortcuts, search: { edit: id } });
        }}
        onClose={() => {
          void navigate({ to: paths.shortcuts, search: {} });
        }}
      />
    );
  },
});
