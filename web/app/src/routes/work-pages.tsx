import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { registerIssueBlocks } from "./issue-blocks.ts";
import { organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";

registerIssueBlocks();

// The screens of work (issue types) are loaded when they are first opened, so the first page stays
// small (coding-standards section 6).

const IssueTypesScreen = lazyRouteComponent(
  () => import("../features/issue-types/index.ts"),
  "IssueTypesPage",
);

const RelationKindsScreen = lazyRouteComponent(
  () => import("../features/relation-kinds/index.ts"),
  "RelationKindsPage",
);

/** *Organization settings › Kinds of relation*: the address says which kind is open in the side panel. */
export const relationKindsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.relationKinds,
  validateSearch: (search: Readonly<Record<string, unknown>>): { open?: string | undefined } => ({
    open: typeof search["open"] === "string" ? search["open"] : undefined,
  }),
  component: function RelationKinds() {
    const { open } = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <OnlyWith path={paths.relationKinds}>
        <RelationKindsScreen
          open={open}
          onOpen={(id: string) => {
            void navigate({ to: paths.relationKinds, search: { open: id } });
          }}
          onClose={() => {
            void navigate({ to: paths.relationKinds, search: {} });
          }}
        />
      </OnlyWith>
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
