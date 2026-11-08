import { t } from "@bugaboo/messages";
import { doNothing, iconName, partOf, part, useShortWindow } from "@bugaboo/ui";
import { useContext } from "react";
import { Dialog, DialogTrigger, OverlayTriggerStateContext, Popover } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import { NavButton } from "./NavItem.tsx";
import { openOverlay } from "./overlay-state.ts";
import { MyAgentsLink, PlaceLink, type PlaceLinkProps } from "./PlaceLink.tsx";
import { toggleSidebar } from "./sidebar-state.ts";

export interface BottomProps extends Pick<PlaceLinkProps, "narrow" | "onNavigate"> {
  /** The sidebar is inside the menu of a tablet: no button to narrow it. */
  readonly inDrawer: boolean;
  /** The person may open *Organization settings*. */
  readonly organizationSettings: boolean;
}

/** Where the menu of the group opens: above its button, from the start edge (words of React Aria). */
const above = "top start";

/** The items of the group: *My settings*, *Organization settings*, the list of shortcuts, and narrowing the bar; where the group is folded, *Timesheet* and *My agents* too. */
function Items({
  narrow,
  inDrawer,
  organizationSettings,
  onNavigate,
  withTimesheet = false,
}: BottomProps & { readonly withTimesheet?: boolean }) {
  return (
    <>
      {withTimesheet ? (
        <PlaceLink path={paths.time} narrow={narrow} onNavigate={onNavigate} />
      ) : null}
      {withTimesheet ? <MyAgentsLink narrow={narrow} onNavigate={onNavigate} /> : null}
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
            onNavigate?.();
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
            onPress={() => {
              onNavigate?.();
              toggleSidebar();
            }}
          />
        </li>
      )}
    </>
  );
}

/** The items inside the popover: each closes the menu once it is used. */
function MenuItems(props: BottomProps) {
  const state = useContext(OverlayTriggerStateContext);
  return (
    <ul {...partOf(part.sidebarList)}>
      <Items
        {...props}
        withTimesheet
        narrow={false}
        onNavigate={() => {
          state?.close();
          props.onNavigate?.();
        }}
      />
    </ul>
  );
}

/**
 * The group at the bottom of the sidebar (*My settings* and the rest). In a window too low for the
 * whole sidebar it is one button, *Settings and more*, whose menu rises from the button over the page,
 * like the menu of the timer, so the sidebar never grows or scrolls and *Sign out* stays in view
 * (M5-F150). The focus goes into the menu, and Esc closes it and gives the focus back.
 */
export function Bottom(props: BottomProps) {
  const short = useShortWindow() && !props.inDrawer;
  if (!short) {
    return (
      <ul {...partOf(part.sidebarBottom)}>
        <Items {...props} />
      </ul>
    );
  }
  return (
    <ul {...partOf(part.sidebarBottom)}>
      <li>
        <DialogTrigger>
          <NavButton
            icon={iconName.more}
            label={t("shell.group-fold")}
            narrow={props.narrow}
            onPress={doNothing}
          />
          <Popover placement={above} {...partOf(part.popover)}>
            <Dialog aria-label={t("shell.group-fold")} {...partOf(part.overlayDialog)}>
              <MenuItems {...props} />
            </Dialog>
          </Popover>
        </DialogTrigger>
      </li>
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
    <div {...partOf(part.sidebarPerson)}>
      {name === undefined || narrow ? null : (
        <span {...partOf(part.sidebarWho)}>{t("shell.signed-in-as", { name })}</span>
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
