import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { NotFoundPage } from "../features/shell/index.ts";
import { paths, routeParam } from "../shared/paths.ts";
import { validateSearch } from "../shared/search-address.ts";
import type { ResultsInput } from "../shared/use-list-view.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";
import { SearchResultsList } from "./search-results.tsx";
import { useSearchNavigation, useSearchState } from "./search-address.ts";

// The search is loaded when it is first opened, so the first page stays small
// (coding-standards section 6).

const SearchScreen = lazyRouteComponent(() => import("../features/search/index.ts"), "SearchPage");
const SavedSearchScreen = lazyRouteComponent(
  () => import("../features/search/index.ts"),
  "SavedSearchPage",
);
/** Loads the issue list under the search bar before the page is drawn, so it does not wait for the page. */
const loadsResults = async (): Promise<void> => {
  await import("../features/issues/index.ts");
};

/** The issue list under the search bar of a page that has no project. */
function results(input: ResultsInput) {
  return <SearchResultsList {...input} />;
}

/** The *Search* page, `/search`: the search over every project the person sees, in `?s=`. */
export const searchRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.search,
  validateSearch,
  loader: loadsScreen(SearchScreen),
  beforeLoad: loadsResults,
  component: function Search() {
    const state = useSearchState();
    const navigateTo = useSearchNavigation();
    return <SearchScreen state={state} onState={navigateTo} results={results} />;
  },
});

/** A saved search, `/s/<ID>`: its own address, which is shared; a change that is not saved is in `?s=`. */
export const savedSearchRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.savedSearch,
  validateSearch,
  loader: loadsScreen(SavedSearchScreen),
  beforeLoad: loadsResults,
  component: function SavedSearch() {
    const id = useAddressPart(routeParam.search);
    const state = useSearchState();
    const navigateTo = useSearchNavigation();
    return id === undefined ? null : (
      <SavedSearchScreen
        id={id}
        state={state}
        onState={navigateTo}
        results={results}
        notFound={<NotFoundPage />}
      />
    );
  },
});
