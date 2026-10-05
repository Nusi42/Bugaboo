import { t } from "@bugaboo/messages";
import { IconButton, iconName, Overlay } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import styles from "./MoreSheet.module.css";
import { NavButton, NavLink } from "./NavItem.tsx";
import { closeOverlay, openOverlay, useOverlay } from "./overlay-state.ts";
import { placeAt } from "./places.ts";

export interface MoreSheetProps {
  readonly administrator: boolean;
  /** The name of the person who is signed in. */
  readonly name: string | undefined;
  readonly onSignOut: () => void;
  readonly signingOut: boolean;
}

/**
 * What the bar of a phone has no room for, as a sheet from the bottom (UX §3.3): *My settings*,
 * *Organization settings*, the list of shortcuts and *Sign out*.
 */
export function MoreSheet({ administrator, name, onSignOut, signingOut }: MoreSheetProps) {
  const overlay = useOverlay("more");
  const mine = placeAt(paths.mySettings);
  const organization = placeAt(paths.organizationSettings);
  return (
    <Overlay {...overlay} label={t("shell.more")} bottom>
      <div className={styles.sheet}>
        <div className={styles.head}>
          <h2 className={styles.title}>{t("shell.more")}</h2>
          <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        </div>
        {name === undefined ? null : (
          <p className={styles.who}>{t("shell.signed-in-as", { name })}</p>
        )}
        <ul className={styles.list}>
          {[mine, administrator ? organization : undefined].map((place) =>
            place === undefined ? null : (
              <li key={place.path}>
                <NavLink
                  href={place.path}
                  icon={place.icon}
                  label={t(place.name)}
                  onNavigate={closeOverlay}
                />
              </li>
            ),
          )}
          <li>
            <NavButton
              icon={iconName.keyboard}
              label={t("shortcut.title")}
              onPress={() => {
                openOverlay("shortcuts");
              }}
            />
          </li>
          <li>
            <NavButton
              icon={iconName["sign-out"]}
              label={t("nav.sign-out")}
              isPending={signingOut}
              onPress={onSignOut}
            />
          </li>
        </ul>
      </div>
    </Overlay>
  );
}
