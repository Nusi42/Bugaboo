import { iconName, part, partOf, type IconName } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { dashboardAddress, projectAddress, savedSearchAddress } from "../../shared/paths.ts";
import { howCurrent, placeCurrent } from "../../shared/places.ts";
import { NavLink, type SidebarSectionView } from "../shell/index.ts";
import { PinMenu } from "./PinMenu.tsx";
import { pinKind, type Pin } from "./pins.ts";

/** Where a pin leads. */
function addressOf(pin: Pin): string {
  if (pin.kind === pinKind.project) {
    return projectAddress(pin.short_name ?? pin.id);
  }
  return pin.kind === pinKind.dashboard ? dashboardAddress(pin.id) : savedSearchAddress(pin.id);
}

/** The icon of a pin: the icon of what it points to, as the sidebar shows it. */
const pinIcon = {
  project: iconName.projects,
  "saved-search": iconName.search,
  dashboard: iconName.dashboards,
} as const satisfies Record<Pin["kind"], IconName>;

/** One pin: its link with its icon and, beside it, its menu (not in the sidebar narrowed to icons). */
export function PinRow({
  pin,
  place,
  pins,
  view,
  menuBox,
  onUnpinned,
}: {
  readonly pin: Pin;
  readonly place: number;
  readonly pins: readonly Pin[];
  readonly view: SidebarSectionView;
  readonly menuBox: (element: HTMLElement | null) => void;
  readonly onUnpinned: () => void;
}) {
  const { pathname } = useLocation();
  const { narrow, onNavigate } = view;
  return (
    <li {...partOf(part.pinnedRow)}>
      <NavLink
        icon={pinIcon[pin.kind]}
        label={pin.name}
        narrow={narrow}
        href={addressOf(pin)}
        current={howCurrent(addressOf(pin), pathname) === placeCurrent.page}
        {...(onNavigate === undefined ? {} : { onNavigate })}
      />
      {narrow ? null : (
        <span {...partOf(part.pinnedMore)} ref={menuBox}>
          <PinMenu pin={pin} place={place} pins={pins} onUnpinned={onUnpinned} />
        </span>
      )}
    </li>
  );
}
