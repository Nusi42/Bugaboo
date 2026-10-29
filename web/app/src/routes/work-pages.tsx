import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import {
  issueAddress,
  layoutsAddress,
  type LayoutsOpen,
  paths,
  projectIssuesAddress,
  routeParam,
  targetParam,
} from "../shared/paths.ts";
import { listReturnFor } from "../shared/list-return.ts";
import { issuePanelAddress, panelClosedAddress, validateSearch } from "../shared/search-address.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { placeEditorRoutes } from "./place-editor-routes.tsx";
import { loadsScreen } from "./preload.ts";
import { useSearchNavigation, useSearchState } from "./search-address.ts";

export { projectIssueTypesRoute, projectLabelsRoute } from "./project-offer-pages.tsx";
export { projectWorkflowsRoute, workflowsRoute } from "./workflow-pages.tsx";
export { savedSearchRoute, searchRoute } from "./search-pages.tsx";

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
  loader: loadsScreen(RelationKindsScreen),
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

const LabelsScreen = lazyRouteComponent(() => import("../features/labels/index.ts"), "LabelsPage");

/** *Organization settings › Labels and tags*: the address says which group, label or tag is open. */
export const labelsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.labels,
  validateSearch: (search: Readonly<Record<string, unknown>>): { open?: string | undefined } => ({
    open: typeof search["open"] === "string" ? search["open"] : undefined,
  }),
  loader: loadsScreen(LabelsScreen),
  component: function Labels() {
    const { open } = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <OnlyWith path={paths.labels}>
        <LabelsScreen
          open={open}
          onOpen={(id: string) => {
            void navigate({ to: paths.labels, search: { open: id } });
          }}
          onClose={() => {
            void navigate({ to: paths.labels, search: {} });
          }}
        />
      </OnlyWith>
    );
  },
});

/** What the address of the editor of layouts says is open: the form and the issue type. */
function layoutsSearch(search: Readonly<Record<string, unknown>>): LayoutsOpen {
  return {
    form: typeof search["form"] === "string" ? search["form"] : undefined,
    type: typeof search["type"] === "string" ? search["type"] : undefined,
  };
}

const layoutRoutes = placeEditorRoutes({
  organizationPath: paths.layouts,
  projectPath: paths.projectLayouts,
  search: layoutsSearch,
  address: layoutsAddress,
  OrganizationScreen: lazyRouteComponent(
    () => import("../features/layouts/index.ts"),
    "OrganizationLayoutsScreen",
  ),
  ProjectScreen: lazyRouteComponent(
    () => import("../features/layouts/index.ts"),
    "ProjectLayoutsScreen",
  ),
});

/** *Organization settings › Layouts*: the forms of the whole organization. */
export const layoutsRoute = layoutRoutes.organization;

/** `/p/WEB/settings/layouts`: the layouts of a project. */
export const projectLayoutsRoute = layoutRoutes.project;

const FieldsScreen = lazyRouteComponent(() => import("../features/fields/index.ts"), "FieldsPage");
const FieldScreen = lazyRouteComponent(() => import("../features/fields/index.ts"), "FieldPage");

/** *Organization settings › Fields*: the own fields, and the form that makes one. */
export const fieldsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.fields,
  loader: loadsScreen(FieldsScreen),
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
  loader: loadsScreen(FieldScreen),
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
  loader: loadsScreen(IssueTypesScreen),
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
  loader: loadsScreen(ProjectIssuesScreen),
  validateSearch,
  component: function ProjectIssues() {
    const project = useAddressPart(routeParam.project);
    const state = useSearchState();
    const onState = useSearchNavigation();
    const navigate = useNavigate();
    return project === undefined ? null : (
      <ProjectIssuesScreen
        address={project}
        state={state}
        onState={onState}
        onFocus={(key: string) => {
          void navigate({ href: issueAddress(key) });
        }}
        onClose={() => {
          void navigate({ href: panelClosedAddress(state) });
        }}
      />
    );
  },
});

/** An issue in the focus view, `/issues/WEB-142`: the address that is shared. */
export const issueRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.issue,
  // Where *Open* of a notification goes on the page: a comment, or the change at a moment.
  validateSearch: (
    search: Readonly<Record<string, unknown>>,
  ): { comment?: string | undefined; change?: string | undefined } => {
    const comment = search[targetParam.comment];
    const change = search[targetParam.change];
    return {
      [targetParam.comment]: typeof comment === "string" ? comment : undefined,
      [targetParam.change]: typeof change === "string" ? change : undefined,
    };
  },
  loader: loadsScreen(IssueScreen),
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
            href:
              project === undefined
                ? paths.home
                : (listReturnFor(project) ?? projectIssuesAddress(project)),
          });
        }}
        onPanel={(project: string, key: string) => {
          void navigate({ href: issuePanelAddress(undefined, project, key) });
        }}
      />
    );
  },
});
