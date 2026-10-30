import { t } from "@bugaboo/messages";
import { useLocation } from "@tanstack/react-router";

import { NavLink } from "./NavItem.tsx";
import { howCurrent, placeAt, placeCurrent, type Place } from "../../shared/places.ts";

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
        current={howCurrent(place.path, pathname) === placeCurrent.page}
        {...(onNavigate === undefined ? {} : { onNavigate })}
      />
    </li>
  );
}
