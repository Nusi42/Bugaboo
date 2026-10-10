import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";

// The screens of work (issue types) are loaded when they are first opened, so the first page stays
// small (coding-standards section 6).

const IssueTypesScreen = lazyRouteComponent(
  () => import("../features/issue-types/index.ts"),
  "IssueTypesPage",
);

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
