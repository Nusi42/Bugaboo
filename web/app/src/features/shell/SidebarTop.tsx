import { t } from "@bugaboo/messages";
import { iconName, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { keysOf } from "../../shared/shortcuts.ts";
import { useUnreadCountText } from "../../shared/unread-notifications.ts";
import { useShortcutOverrides } from "../../shared/use-shortcuts.ts";
import { Keys } from "./Keys.tsx";
import { NavButton } from "./NavItem.tsx";
import { openOverlay } from "./overlay-state.ts";
import { PlaceLink, type PlaceLinkProps } from "./PlaceLink.tsx";

/** Search and the start page. */
export function Top({ narrow, onNavigate }: Pick<PlaceLinkProps, "narrow" | "onNavigate">) {
  const overrides = useShortcutOverrides();
  const unread = useUnreadCountText();
  const searchKeys = keysOf("quick-search", overrides)[0] ?? "";
  const newKeys = keysOf("new-issue", overrides)[0] ?? "";
  return (
    <ul {...partOf(part.sidebarList)}>
      <li>
        <NavButton
          icon={iconName.search}
          label={t("shell.search")}
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
    </ul>
  );
}
