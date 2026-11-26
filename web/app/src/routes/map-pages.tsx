import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import type { IssueMapAsk } from "../features/relation-map/index.ts";
import { paths, routeParam } from "../shared/paths.ts";
import {
  kindsShownOf,
  kindsShownParts,
  mapParam,
  stepChoices,
} from "../shared/relation-map-view.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";

// The relation map is its own chunk, loaded when a map is first opened (coding-standards §6).
const IssueMapScreen = lazyRouteComponent(
  () => import("../features/relation-map/index.ts"),
  "IssueMapPage",
);

/** What the address says of a map: the steps, the kinds turned off, and `parts=0` for *part of* turned off. */
interface MapSearch {
  readonly steps?: number | undefined;
  readonly hide?: string | undefined;
  readonly parts?: string | undefined;
}

/** The ask of a map from its address; anything it does not understand is left at the start. */
function askOf(search: MapSearch): IssueMapAsk {
  // *Everything* is the address without steps; anything else unknown is *Everything* too.
  const steps = stepChoices.find((each) => each !== undefined && each === search.steps);
  return { steps, shown: kindsShownOf(search.hide, search.parts) };
}

/** The address of a map with this ask: only what differs from the start. */
function searchOf(ask: IssueMapAsk): MapSearch {
  return {
    ...(ask.steps === undefined ? {} : { [mapParam.steps]: ask.steps }),
    ...kindsShownParts(ask.shown),
  };
}

/** A part of the address as text: the router reads `0` as a number. */
function textIn(value: unknown): string | undefined {
  if (typeof value === "string") {
    return value;
  }
  return typeof value === "number" ? String(value) : undefined;
}

/** The relation map of an issue, `/issues/WEB-142/map` (UX §2.1, §4.15). */
export const issueMapRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.issueMap,
  validateSearch: (search: Readonly<Record<string, unknown>>): MapSearch => {
    const steps = search[mapParam.steps];
    return {
      [mapParam.steps]: typeof steps === "number" ? steps : undefined,
      [mapParam.hide]: textIn(search[mapParam.hide]),
      [mapParam.partOf]: textIn(search[mapParam.partOf]),
    };
  },
  loader: loadsScreen(IssueMapScreen),
  component: function IssueMap() {
    const issue = useAddressPart(routeParam.issue);
    const search: MapSearch = useSearch({ strict: false });
    const navigate = useNavigate();
    return issue === undefined ? null : (
      <IssueMapScreen
        address={issue}
        ask={askOf(search)}
        onAsk={(key: string, ask: IssueMapAsk) => {
          void navigate({ to: paths.issueMap, params: { issue: key }, search: searchOf(ask) });
        }}
      />
    );
  },
});
