import { t } from "@bugaboo/messages";
import { useLocation } from "@tanstack/react-router";

import { isAtOrUnder, paths } from "../../shared/paths.ts";
import { NavLink } from "./NavItem.tsx";
import { placeAt, type Place } from "../../shared/places.ts";

/** Whether a page of the menu is the one that is open: *My settings* stays marked on its topics. */
function isCurrent(path: string, pathname: string): boolean {
  if (path === paths.home) {
    return pathname === path;
  }
  return isAtOrUnder(path, pathname);
}

export interface PlaceLinkProps {
  readonly path: Place["path"];
  readonly narrow: boolean;
  readonly onNavigate: (() => void) | undefined;
  /** A number after the words, such as the notifications not read yet. */
  readonly count?: string | undefined;
}

/** The link to a screen of the app, as a list item. */
export function PlaceLink({ path, narrow, onNavigate, count }: PlaceLinkProps) {
  const { pathname } = useLocation();
  const place = placeAt(path);
  if (place === undefined) {
    return null;
  }
  return (
    <li>
      <NavLink
        href={place.path}
        icon={place.icon}
        label={t(place.name)}
        narrow={narrow}
        count={count}
        current={isCurrent(place.path, pathname)}
        {...(onNavigate === undefined ? {} : { onNavigate })}
      />
    </li>
  );
}
