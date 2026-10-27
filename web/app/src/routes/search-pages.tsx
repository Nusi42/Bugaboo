import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { NotFoundPage } from "../features/shell/index.ts";
import { paths, routeParam } from "../shared/paths.ts";
import { validateSearch } from "../shared/search-address.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";
import { useSearchNavigation, useSearchState } from "./search-address.ts";

// The search is loaded when it is first opened, so the first page stays small
// (coding-standards section 6).

const SearchScreen = lazyRouteComponent(() => import("../features/search/index.ts"), "SearchPage");
const SavedSearchScreen = lazyRouteComponent(
  () => import("../features/search/index.ts"),
  "SavedSearchPage",
);

/** The *Search* page, `/search`: the search over every project the person sees, in `?s=`. */
export const searchRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.search,
  validateSearch,
  loader: loadsScreen(SearchScreen),
  component: function Search() {
    const state = useSearchState();
    const navigateTo = useSearchNavigation();
    return <SearchScreen state={state} onState={navigateTo} />;
  },
});

/** A saved search, `/s/<ID>`: its own address, which is shared; a change that is not saved is in `?s=`. */
export const savedSearchRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.savedSearch,
  validateSearch,
  loader: loadsScreen(SavedSearchScreen),
  component: function SavedSearch() {
    const id = useAddressPart(routeParam.search);
    const state = useSearchState();
    const navigateTo = useSearchNavigation();
    return id === undefined ? null : (
      <SavedSearchScreen id={id} state={state} onState={navigateTo} notFound={<NotFoundPage />} />
    );
  },
});
