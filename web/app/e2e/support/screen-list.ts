import { type ScreenValues } from "./every-screen.ts";
import { filled, paths, routeParam, type Path } from "../../src/shared/paths.ts";
import { guestScreens, places } from "../../src/shared/places.ts";
import { searchHref } from "../../src/shared/search-address.ts";

/** One screen of the route table, at an address that opens it on the sample. */
export interface ScreenAddress {
  /** The route, as the table of screens names it (`/settings/people/$person`), and the view it shows, if not its own. */
  readonly path: string;
  readonly address: string;
}

/** The address of a route with its changing part filled in (no route has two); the page of a personal theme opens one. */
function addressOf(path: Path, values: ScreenValues): string {
  const param = Object.values(routeParam).find((each) => filled(path, each, "") !== path);
  if (param === undefined) {
    return path;
  }
  return filled(path, param, path === paths.myTheme ? values.personalTheme : values[param]);
}

/**
 * The views a screen of the route table shows besides its own: the issues of a project and the page *Search* as a
 * timeline (`show=timeline`, UX §4.7), which the table does not list because they are views, not screens.
 */
function everyView(values: ScreenValues): readonly ScreenAddress[] {
  const timeline = "timeline";
  return [
    {
      path: `${paths.projectIssues} ${timeline}`,
      address: searchHref({ text: "", project: values.project, view: timeline }),
    },
    {
      path: `${paths.search} ${timeline}`,
      address: searchHref({ text: "", everything: true, view: timeline }),
    },
  ];
}

/** Every screen of the route table (`places`), at an address on the sample, then the views they show. */
export function everyScreen(values: ScreenValues): readonly ScreenAddress[] {
  return [
    ...places.map((place) => ({ path: place.path, address: addressOf(place.path, values) })),
    ...everyView(values),
  ];
}

/** The screens before sign-in (`guestScreens`), at their own addresses: each shows what it shows to nobody signed in. */
export function everyGuestScreen(): readonly ScreenAddress[] {
  return guestScreens.map((path) => ({ path, address: path }));
}
