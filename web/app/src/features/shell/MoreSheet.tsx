import { t } from "@bugaboo/messages";
import { IconButton, iconName, Overlay, overlayEdge, partOf, part } from "@bugaboo/ui";

import { Fragment } from "react";

import { paths } from "../../shared/paths.ts";
import { NavButton, NavLink } from "./NavItem.tsx";
import { closeOverlay, openOverlay, useOverlay } from "./overlay-state.ts";
import { placeAt } from "../../shared/places.ts";
import { useSidebarSections } from "./sidebar-sections.ts";
import { useMyAgentsEntry } from "../../shared/use-my-agents-entry.ts";

export interface MoreSheetProps {
  /** The person may open *Organization settings*. */
  readonly organizationSettings: boolean;
  /** The name of the person who is signed in. */
  readonly name: string | undefined;
  readonly onSignOut: () => void;
  readonly signingOut: boolean;
}

/** *My agents* for a person who has an agent or may create one, *My settings*, and *Organization settings* for a person who may open it. */
function Places({ organizationSettings }: Pick<MoreSheetProps, "organizationSettings">) {
  const agents = useMyAgentsEntry();
  const places = [
    agents ? placeAt(paths.myAgents) : undefined,
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
      <div {...partOf(part.shellSheet)}>
        <div {...partOf(part.shellSheetHead)}>
          <h2 {...partOf(part.shellTitle)}>{t("shell.more")}</h2>
          <IconButton icon={iconName.close} label={t("common.close")} onPress={closeOverlay} />
        </div>
        {name === undefined ? null : (
          <p {...partOf(part.moreSheetWho)}>{t("shell.signed-in-as", { name })}</p>
        )}
        {sections.map((section) => (
          <Fragment key={section.heading}>
            {section.render({ narrow: false, place: "sheet", onNavigate: closeOverlay })}
          </Fragment>
        ))}
        <ul {...partOf(part.list, { variants: ["tight"] })}>
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
