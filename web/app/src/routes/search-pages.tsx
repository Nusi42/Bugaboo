import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { NotFoundPage } from "../features/shell/index.ts";
import { paths, routeParam, savedSearchAddress } from "../shared/paths.ts";
import { readSearchAddress, searchHref, type SearchAddress } from "../shared/search-address.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";

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
  validateSearch: readSearchAddress,
  loader: loadsScreen(SearchScreen),
  component: function Search() {
    const address: SearchAddress = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <SearchScreen
        address={address}
        onAddress={(next: SearchAddress) => {
          // Typing changes the search many times: it replaces the address, so back is not a step per letter.
          void navigate({ to: paths.search, search: next, replace: true });
        }}
      />
    );
  },
});

/** A saved search, `/s/<ID>`: its own address, which is shared; a change that is not saved is in `?s=`. */
export const savedSearchRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.savedSearch,
  validateSearch: readSearchAddress,
  loader: loadsScreen(SavedSearchScreen),
  component: function SavedSearch() {
    const id = useAddressPart(routeParam.search);
    const address: SearchAddress = useSearch({ strict: false });
    const navigate = useNavigate();
    return id === undefined ? null : (
      <SavedSearchScreen
        id={id}
        address={address}
        notFound={<NotFoundPage />}
        onAddress={(next: SearchAddress) => {
          void navigate({ href: searchHref(savedSearchAddress(id), next), replace: true });
        }}
      />
    );
  },
});
