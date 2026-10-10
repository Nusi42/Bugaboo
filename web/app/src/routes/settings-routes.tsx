import { useNavigate, useSearch } from "@tanstack/react-router";

import { StartPage } from "../features/my-settings/index.ts";
import { ShortcutsPage, useQuickResults } from "../features/shell/index.ts";
import { paths } from "../shared/paths.ts";

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
  const pages = groups.flatMap((group) =>
    group.results.map((result) => ({
      path: result.path,
      title: result.hint === undefined ? result.title : `${result.title} (${result.hint})`,
    })),
  );
  return <StartPage pages={pages} />;
}
