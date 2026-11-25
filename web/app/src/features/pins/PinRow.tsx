import { iconName, part, partOf } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { projectAddress, savedSearchAddress } from "../../shared/paths.ts";
import { howCurrent, placeCurrent } from "../../shared/places.ts";
import { NavLink, type SidebarSectionView } from "../shell/index.ts";
import { PinMenu } from "./PinMenu.tsx";
import { pinKind, type Pin } from "./pins.ts";

/** Where a pin leads. */
function addressOf(pin: Pin): string {
  return pin.kind === pinKind.project
    ? projectAddress(pin.short_name ?? pin.id)
    : savedSearchAddress(pin.id);
}

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
        icon={pin.kind === pinKind.project ? iconName.projects : iconName.search}
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
