import { t } from "@bugaboo/messages";
import { IconButton, iconName, Overlay, overlayEdge, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { NavButton, NavLink } from "./NavItem.tsx";
import { closeOverlay, openOverlay, useOverlay } from "./overlay-state.ts";
import { placeAt } from "../../shared/places.ts";
import { useSidebarSections } from "./sidebar-sections.ts";

export interface MoreSheetProps {
  /** The person may open *Organization settings*. */
  readonly organizationSettings: boolean;
  /** The name of the person who is signed in. */
  readonly name: string | undefined;
  readonly onSignOut: () => void;
  readonly signingOut: boolean;
}

/** *My settings*, and *Organization settings* for a person who may open it. */
function Places({ organizationSettings }: Pick<MoreSheetProps, "organizationSettings">) {
  const places = [
    placeAt(paths.mySettings),
    organizationSettings ? placeAt(paths.organizationSettings) : undefined,
  ];
  return places.map((place) =>
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
  );
}

/**
 * What the bar of a phone has no room for, as a sheet from the bottom (UX §3.3): *My settings*,
 * *Organization settings*, the list of shortcuts and *Sign out*.
 */
export function MoreSheet({ organizationSettings, name, onSignOut, signingOut }: MoreSheetProps) {
  const overlay = useOverlay("more");
  const sections = useSidebarSections();
  return (
    <Overlay {...overlay} label={t("shell.more")} from={overlayEdge.bottom}>
      <div {...partOf(part.moreSheet)}>
        <div {...partOf(part.moreSheetHead)}>
          <h2 {...partOf(part.moreSheetTitle)}>{t("shell.more")}</h2>
          <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        </div>
        {name === undefined ? null : (
          <p {...partOf(part.moreSheetWho)}>{t("shell.signed-in-as", { name })}</p>
        )}
        {sections.map((section) => (
          <section
            key={section.heading}
            {...partOf(part.moreSheetSection)}
            aria-label={t(section.heading)}
          >
            <h3 {...partOf(part.moreSheetSectionHeading)}>{t(section.heading)}</h3>
            {section.render({ narrow: false, onNavigate: closeOverlay })}
          </section>
        ))}
        <ul {...partOf(part.moreSheetList)}>
          <Places organizationSettings={organizationSettings} />
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
