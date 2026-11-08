import { t } from "@bugaboo/messages";
import { iconName, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { keysOf } from "../../shared/shortcuts.ts";
import { useUnreadCountText } from "../../shared/unread-notifications.ts";
import { useShortcutOverrides } from "../../shared/use-shortcuts.ts";
import { Keys } from "./Keys.tsx";
import { NavButton } from "./NavItem.tsx";
import { openOverlay } from "./overlay-state.ts";
import { MyAgentsLink, PlaceLink, type PlaceLinkProps } from "./PlaceLink.tsx";

/**
 * Quick search, a new issue, the start page, notifications, the timesheet, *My agents* (for a person who has an agent or
 * may create one) and the page of Search. `folded`: the group at the bottom is one button with the timesheet and *My
 * agents* in its menu.
 */
export function Top({
  narrow,
  onNavigate,
  folded,
}: Pick<PlaceLinkProps, "narrow" | "onNavigate"> & { readonly folded: boolean }) {
  const overrides = useShortcutOverrides();
  const unread = useUnreadCountText();
  const searchKeys = keysOf("quick-search", overrides)[0] ?? "";
  const newKeys = keysOf("new-issue", overrides)[0] ?? "";
  return (
    <ul {...partOf(part.sidebarList)}>
      <li>
        <NavButton
          icon={iconName.search}
          label={t("shell.quick-search")}
          narrow={narrow}
          hint={<Keys sequence={searchKeys} />}
          onPress={() => {
            openOverlay("search");
          }}
        />
      </li>
      <li>
        <NavButton
          icon={iconName.plus}
          label={t("issues.new")}
          narrow={narrow}
          hint={<Keys sequence={newKeys} />}
          onPress={() => {
            openOverlay("new-issue");
          }}
        />
      </li>
      <PlaceLink path={paths.home} narrow={narrow} onNavigate={onNavigate} />
      <PlaceLink
        path={paths.notifications}
        narrow={narrow}
        onNavigate={onNavigate}
        count={unread}
      />
      {folded ? null : <PlaceLink path={paths.time} narrow={narrow} onNavigate={onNavigate} />}
      {folded ? null : <MyAgentsLink narrow={narrow} onNavigate={onNavigate} />}
      <PlaceLink path={paths.search} narrow={narrow} onNavigate={onNavigate} />
    </ul>
  );
}
