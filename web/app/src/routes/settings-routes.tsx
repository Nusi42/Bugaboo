import { useQuickResults } from "../features/shell/index.ts";
import { StartScreen } from "./start-screen.ts";

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
