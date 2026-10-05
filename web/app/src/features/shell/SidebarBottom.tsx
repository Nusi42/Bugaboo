import { t } from "@bugaboo/messages";
import { iconName } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { NavButton } from "./NavItem.tsx";
import { openOverlay } from "./overlay-state.ts";
import { PlaceLink, type PlaceLinkProps } from "./PlaceLink.tsx";
import styles from "./Sidebar.module.css";
import { toggleSidebar } from "./sidebar-state.ts";

export interface BottomProps extends Pick<PlaceLinkProps, "narrow" | "onNavigate"> {
  /** The sidebar is inside the menu of a tablet: no button to narrow it. */
  readonly inDrawer: boolean;
  /** The person may open *Organization settings*. */
  readonly organizationSettings: boolean;
}

/** *My settings*, *Organization settings*, the list of shortcuts and the button to narrow the bar. */
export function Bottom({ narrow, inDrawer, organizationSettings, onNavigate }: BottomProps) {
  return (
    <ul className={styles.bottom}>
      <PlaceLink path={paths.mySettings} narrow={narrow} onNavigate={onNavigate} />
      {organizationSettings ? (
        <PlaceLink path={paths.organizationSettings} narrow={narrow} onNavigate={onNavigate} />
      ) : null}
      <li>
        <NavButton
          icon={iconName.keyboard}
          label={t("shortcut.title")}
          narrow={narrow}
          onPress={() => {
            openOverlay("shortcuts");
          }}
        />
      </li>
      {inDrawer ? null : (
        <li>
          <NavButton
            icon={narrow ? iconName.widen : iconName.narrow}
            label={narrow ? t("shell.widen") : t("shell.narrow")}
            narrow={narrow}
            onPress={toggleSidebar}
          />
        </li>
      )}
    </ul>
  );
}

export interface PersonProps {
  readonly narrow: boolean;
  /** The name of the person who is signed in. */
  readonly name: string | undefined;
  readonly onSignOut: () => void;
  readonly signingOut: boolean;
}

/** Who is signed in, and the way to sign out. */
export function Person({ narrow, name, onSignOut, signingOut }: PersonProps) {
  return (
    <div className={styles.person}>
      {name === undefined || narrow ? null : (
        <span className={styles.who}>{t("shell.signed-in-as", { name })}</span>
      )}
      <NavButton
        icon={iconName["sign-out"]}
        label={t("nav.sign-out")}
        narrow={narrow}
        isPending={signingOut}
        onPress={onSignOut}
      />
    </div>
  );
}
