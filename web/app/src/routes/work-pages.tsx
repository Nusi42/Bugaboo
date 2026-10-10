import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import {
  issueAddress,
  issuePanelAddress,
  paths,
  projectIssuesAddress,
  routeParam,
} from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, organizationSettingsRoute } from "./layouts.tsx";
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

// The screens of issues are loaded when they are first opened, so the first page stays small
// (coding-standards section 6).

const ProjectIssuesScreen = lazyRouteComponent(
  () => import("../features/issues/index.ts"),
  "ProjectIssuesPage",
);
const IssueScreen = lazyRouteComponent(
  () => import("../features/issues/index.ts"),
  "IssueFocusPage",
);

/** The issues of a project, `/p/WEB/issues`; `?open=WEB-142` shows one in the side panel. */
export const projectIssuesRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectIssues,
  validateSearch: (search: Readonly<Record<string, unknown>>): { open?: string | undefined } => ({
    open: typeof search["open"] === "string" ? search["open"] : undefined,
  }),
  component: function ProjectIssues() {
    const project = useAddressPart(routeParam.project);
    const { open } = useSearch({ strict: false });
    const navigate = useNavigate();
    return project === undefined ? null : (
      <ProjectIssuesScreen
        address={project}
        open={open}
        onFocus={(key: string) => {
          void navigate({ href: issueAddress(key) });
        }}
        onClose={() => {
          void navigate({ href: projectIssuesAddress(project) });
        }}
      />
    );
  },
});

/** An issue in the focus view, `/issues/WEB-142`: the address that is shared. */
export const issueRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.issue,
  component: function Issue() {
    const issue = useAddressPart(routeParam.issue);
    const navigate = useNavigate();
    return issue === undefined ? null : (
      <IssueScreen
        address={issue}
        onFocus={(key: string) => {
          void navigate({ href: issueAddress(key), replace: true });
        }}
        onBack={(project: string | undefined) => {
          void navigate({
            href: project === undefined ? paths.home : projectIssuesAddress(project),
          });
        }}
        onPanel={(project: string, key: string) => {
          void navigate({ href: issuePanelAddress(project, key) });
        }}
      />
    );
  },
});
