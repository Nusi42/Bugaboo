import { createRoute, useNavigate, useSearch } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

/** What the screens of an editor of a place are given: what is open, and how to open another. */
interface PlaceScreenProps<Open> {
  readonly open: Open;
  readonly onOpen: (shortName: string | undefined, open: Open) => void;
}

/** A screen that is loaded when its route is first opened: a component, and the way to load it early. */
type Screen<Props> = ((props: Props) => ReactNode) & {
  readonly preload?: (() => Promise<void> | undefined) | undefined;
};

/** What the routes of an editor that works on a place (layouts, workflows) are made of. */
export interface PlaceEditorRoutes<Open, Organization extends string, Project extends string> {
  /** The path of the screen of the whole organization, and the one of a project. */
  readonly organizationPath: Organization;
  readonly projectPath: Project;
  /** What the address says is open (the form, the issue type), read from its search part. */
  readonly search: (search: Readonly<Record<string, unknown>>) => Open;
  /** The address of what is open at a place. */
  readonly address: (shortName: string | undefined, open: Open) => string;
  readonly OrganizationScreen: Screen<PlaceScreenProps<Open>>;
  readonly ProjectScreen: Screen<PlaceScreenProps<Open> & { readonly address: string }>;
}

/**
 * The two routes of an editor that works on a place, written once: *Organization settings* and the
 * settings of a project. The layouts and the workflows have the same two, and differ only in what
 * is open and which screens show it.
 */
export function placeEditorRoutes<Open, Organization extends string, Project extends string>(
  config: Readonly<PlaceEditorRoutes<Open, Organization, Project>>,
) {
  const { OrganizationScreen, ProjectScreen } = config;
  const organization = createRoute({
    getParentRoute: () => organizationSettingsRoute,
    path: config.organizationPath,
    validateSearch: config.search,
    loader: loadsScreen(OrganizationScreen),
    component: function OrganizationEditor() {
      const open = useSearch({ strict: false });
      const navigate = useNavigate();
      return (
        <OnlyWith path={config.organizationPath}>
          <OrganizationScreen
            open={config.search(open)}
            onOpen={(_, next: Open) => {
              void navigate({ href: config.address(undefined, next) });
            }}
          />
        </OnlyWith>
      );
    },
  });
  const project = createRoute({
    getParentRoute: () => appRoute,
    path: config.projectPath,
    validateSearch: config.search,
    loader: loadsScreen(ProjectScreen),
    component: function ProjectEditor() {
      const address = useAddressPart(routeParam.project);
      const open = useSearch({ strict: false });
      const navigate = useNavigate();
      return address === undefined ? null : (
        <ProjectScreen
          address={address}
          open={config.search(open)}
          onOpen={(shortName: string | undefined, next: Open) => {
            void navigate({ href: config.address(shortName ?? address, next) });
          }}
        />
      );
    },
  });
  return { organization, project };
}
