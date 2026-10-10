import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";

// The screens of work (issue types) are loaded when they are first opened, so the first page stays
// small (coding-standards section 6).

const IssueTypesScreen = lazyRouteComponent(
  () => import("../features/issue-types/index.ts"),
  "IssueTypesPage",
);

const FieldsScreen = lazyRouteComponent(() => import("../features/fields/index.ts"), "FieldsPage");
const FieldScreen = lazyRouteComponent(() => import("../features/fields/index.ts"), "FieldPage");

/** *Organization settings › Fields*: the own fields, and the form that makes one. */
export const fieldsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.fields,
  component: function Fields() {
    return (
      <OnlyWith path={paths.fields}>
        <FieldsScreen />
      </OnlyWith>
    );
  },
});

/** The page of one own field, `/settings/fields/<ID>`. */
export const fieldRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.field,
  component: function FieldOfTheAddress() {
    const id = useAddressPart(routeParam.field);
    return (
      <OnlyWith path={paths.field}>{id === undefined ? null : <FieldScreen field={id} />}</OnlyWith>
    );
  },
});

/** *Organization settings › Issue types*: the address says which type is open in the side panel. */
export const issueTypesRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.issueTypes,
  validateSearch: (search: Readonly<Record<string, unknown>>): { open?: string | undefined } => ({
    open: typeof search["open"] === "string" ? search["open"] : undefined,
  }),
  component: function IssueTypes() {
    const { open } = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <OnlyWith path={paths.issueTypes}>
        <IssueTypesScreen
          open={open}
          onOpen={(id: string) => {
            void navigate({ to: paths.issueTypes, search: { open: id } });
          }}
          onClose={() => {
            void navigate({ to: paths.issueTypes, search: {} });
          }}
        />
      </OnlyWith>
    );
  },
});
