import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import type { IssueMapAsk } from "../features/relation-map/index.ts";
import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";

// The relation map is its own chunk, loaded when a map is first opened (coding-standards §6).
const IssueMapScreen = lazyRouteComponent(
  () => import("../features/relation-map/index.ts"),
  "IssueMapPage",
);

/** What the address of a relation map says besides its start: the steps away, the kinds turned off, and *part of*. */
const mapParam = { steps: "steps", hide: "hide", partOf: "parts" } as const;

/** The steps a map can be asked for in its address; *Everything* is the address without them. */
const askedSteps = [1, 2, 3] as const;

/** What separates the kinds turned off in the address. */
const listSeparator = ",";

/** What the address says of a map: the steps, the kinds turned off, and `parts=0` for *part of* turned off. */
interface MapSearch {
  readonly steps?: number | undefined;
  readonly hide?: string | undefined;
  readonly parts?: number | undefined;
}

/** The ask of a map from its address; anything it does not understand is left at the start. */
function askOf(search: MapSearch): IssueMapAsk {
  const steps = askedSteps.find((each) => each === search.steps);
  const hidden =
    search.hide === undefined || search.hide === "" ? [] : search.hide.split(listSeparator);
  return { steps, shown: { hidden, partOf: search.parts !== 0 } };
}

/** The address of a map with this ask: only what differs from the start. */
function searchOf(ask: IssueMapAsk): MapSearch {
  return {
    ...(ask.steps === undefined ? {} : { [mapParam.steps]: ask.steps }),
    ...(ask.shown.hidden.length === 0
      ? {}
      : { [mapParam.hide]: ask.shown.hidden.join(listSeparator) }),
    ...(ask.shown.partOf ? {} : { [mapParam.partOf]: 0 }),
  };
}

/** A number in the address, or nothing. */
function numberIn(value: unknown): number | undefined {
  return typeof value === "number" ? value : undefined;
}

/** The relation map of an issue, `/issues/WEB-142/map` (UX §2.1, §4.15). */
export const issueMapRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.issueMap,
  validateSearch: (search: Readonly<Record<string, unknown>>): MapSearch => {
    const hide = search[mapParam.hide];
    return {
      [mapParam.steps]: numberIn(search[mapParam.steps]),
      [mapParam.hide]: typeof hide === "string" ? hide : undefined,
      [mapParam.partOf]: numberIn(search[mapParam.partOf]),
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
