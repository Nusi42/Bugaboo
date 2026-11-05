import { t } from "@bugaboo/messages";
import { type IconName, partOf, part, ariaCurrent } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { Button, Link } from "react-aria-components";

import { type Path } from "../../shared/paths.ts";
import { howCurrent, placeAt } from "../../shared/places.ts";
import { BarPlace } from "./BarPlace.tsx";
import { openOverlay, type OverlayName } from "./overlay-state.ts";

/** A link of the bar to a page of the app. */
export function PlaceTab({
  path,
  count,
}: {
  readonly path: Path;
  readonly count?: string | undefined;
}) {
  const { pathname } = useLocation();
  const place = placeAt(path);
  if (place === undefined) {
    return null;
  }
  const current = howCurrent(place.path, pathname) !== undefined;
  return (
    <li {...partOf(part.bottomBarTab)}>
      <Link
        {...partOf(part.bottomBarItem, { states: [current && "current"] })}
        href={place.path}
        {...(current ? { "aria-current": ariaCurrent.page } : {})}
      >
        <BarPlace icon={place.icon} count={count}>
          {t(place.name)}
        </BarPlace>
      </Link>
    </li>
  );
}

interface OverlayTabProps {
  readonly icon: IconName;
  readonly label: string;
  /** The layer the button opens. */
  readonly opens: OverlayName;
  /** The layer is open now (for screen readers). */
  readonly expanded?: boolean;
  /** The place the tab stands for (*Search*): the tab is current there, as the link of a place is. */
  readonly place?: Path;
}

/** A button of the bar that opens a layer over the page. */
export function OverlayTab({ icon, label, opens, expanded, place }: OverlayTabProps) {
  const { pathname } = useLocation();
  const current = place !== undefined && howCurrent(place, pathname) !== undefined;
  return (
    <li {...partOf(part.bottomBarTab)}>
      <Button
        {...partOf(part.bottomBarItem, { states: [current && "current"] })}
        {...(current ? { "aria-current": ariaCurrent.page } : {})}
        {...(expanded === undefined ? {} : { "aria-expanded": expanded })}
        onPress={() => {
          openOverlay(opens);
        }}
      >
        <BarPlace icon={icon}>{label}</BarPlace>
      </Button>
    </li>
  );
}
