import { t } from "@bugaboo/messages";

import { Sections } from "./Sections.tsx";
import styles from "./Sidebar.module.css";
import { Bottom, Person, type PersonProps } from "./SidebarBottom.tsx";
import { Top } from "./SidebarTop.tsx";

export interface SidebarProps extends PersonProps {
  /** The sidebar is inside the menu of a tablet: no button to narrow it. */
  readonly inDrawer?: boolean;
  readonly administrator: boolean;
  /** Called after a link was used, for a menu that closes. */
  readonly onNavigate?: () => void;
}

/**
 * The sidebar of a computer, and the menu a tablet slides in (UX §3.1, §3.2): search, the start
 * page, what features add (the project tree), and at the bottom *My settings*, *Organization
 * settings* for administrators, the list of shortcuts and *Sign out*.
 */
export function Sidebar({
  narrow,
  inDrawer = false,
  administrator,
  name,
  onSignOut,
  signingOut,
  onNavigate,
}: SidebarProps) {
  return (
    <nav
      className={styles.sidebar}
      data-narrow={narrow ? "" : undefined}
      data-drawer={inDrawer ? "" : undefined}
      aria-label={t("shell.menu")}
    >
      <span className={narrow ? styles.brandNarrow : styles.brand}>{t("app.name")}</span>
      <Top narrow={narrow} onNavigate={onNavigate} />
      <Sections narrow={narrow} />
      <Bottom
        narrow={narrow}
        inDrawer={inDrawer}
        administrator={administrator}
        onNavigate={onNavigate}
      />
      <Person narrow={narrow} name={name} onSignOut={onSignOut} signingOut={signingOut} />
    </nav>
  );
}
