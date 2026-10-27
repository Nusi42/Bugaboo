import { useNavigate, useSearch } from "@tanstack/react-router";

import { ShortcutsPage, useQuickResults } from "../features/shell/index.ts";
import { paths } from "../shared/paths.ts";
import { StartScreen } from "./start-screen.ts";

/** The page of the keyboard shortcuts, with the address saying which one is open. */
export function ShortcutsRoute() {
  const { edit } = useSearch({ strict: false });
  const navigate = useNavigate();
  return (
    <ShortcutsPage
      edit={edit}
      onEdit={(id) => {
        void navigate({ to: paths.shortcuts, search: { edit: id } });
      }}
      onClose={() => {
        void navigate({ to: paths.shortcuts, search: {} });
      }}
    />
  );
}

/** The start page, with the pages quick search knows to choose from (UX §4.13). */
export function StartPageRoute() {
  const { groups } = useQuickResults("");
  // Only what opens a page can be a start page: an action has no address.
  const pages = groups.flatMap((group) =>
    group.results.flatMap((result) =>
      result.path === undefined
        ? []
        : [
            {
              path: result.path,
              title: result.hint === undefined ? result.title : `${result.title} (${result.hint})`,
            },
          ],
    ),
  );
  return <StartScreen pages={pages} />;
}
