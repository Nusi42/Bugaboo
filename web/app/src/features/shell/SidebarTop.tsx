import { t } from "@bugaboo/messages";
import { iconName } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { keysOf } from "../../shared/shortcuts.ts";
import { useShortcutOverrides } from "../../shared/use-shortcuts.ts";
import { Keys } from "./Keys.tsx";
import { NavButton } from "./NavItem.tsx";
import { openOverlay } from "./overlay-state.ts";
import { PlaceLink, type PlaceLinkProps } from "./PlaceLink.tsx";
import styles from "./Sidebar.module.css";

/** Search and the start page. */
export function Top({ narrow, onNavigate }: Pick<PlaceLinkProps, "narrow" | "onNavigate">) {
  const searchKeys = keysOf("quick-search", useShortcutOverrides())[0] ?? "";
  return (
    <ul className={styles.list}>
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
      <PlaceLink path={paths.home} narrow={narrow} onNavigate={onNavigate} />
    </ul>
  );
}
