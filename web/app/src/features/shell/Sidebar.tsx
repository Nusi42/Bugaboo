import { t } from "@bugaboo/messages";
import { partOf, part, useShortWindow } from "@bugaboo/ui";

import { Sections } from "./Sections.tsx";
import { timePlace } from "./time-parts.ts";
import { TimeSlot } from "./TimeSlot.tsx";
import { Bottom, Person, type PersonProps } from "./SidebarBottom.tsx";
import { Top } from "./SidebarTop.tsx";

export interface SidebarProps extends PersonProps {
  /** The sidebar is inside the menu of a tablet: no button to narrow it. */
  readonly inDrawer?: boolean;
  /** A screen has narrowed the sidebar; the button that would widen it is not shown. */
  readonly locked?: boolean;
  /** The person may open *Organization settings*. */
  readonly organizationSettings: boolean;
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
  locked = false,
  organizationSettings,
  name,
  onSignOut,
  signingOut,
  onNavigate,
}: SidebarProps) {
  // In a low window the sections stand closer, so the timer and the work day fit (M5-F150).
  const short = useShortWindow() && !inDrawer;
  // Where the group at the bottom is one button, *Timesheet* is in its menu: the sidebar stays low.
  const folded = useShortWindow() && !(inDrawer || locked);
  return (
    <nav
      {...partOf(part.sidebar, {
        variants: [narrow && "narrow", inDrawer && "drawer", short && "short"],
      })}
      aria-label={t("shell.menu")}
    >
      <span {...partOf(part.sidebarBrand, { variants: [narrow && "narrow"] })}>
        {t("app.name")}
      </span>
      <Top narrow={narrow} onNavigate={onNavigate} folded={folded} />
      <Sections narrow={narrow} onNavigate={onNavigate} />
      <TimeSlot place={timePlace.sidebar} narrow={narrow} />
      <Bottom
        narrow={narrow}
        inDrawer={inDrawer || locked}
        organizationSettings={organizationSettings}
        onNavigate={onNavigate}
      />
      <Person narrow={narrow} name={name} onSignOut={onSignOut} signingOut={signingOut} />
    </nav>
  );
}
