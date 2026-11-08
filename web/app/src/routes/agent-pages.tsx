import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

// The pages of agents are loaded when they are first opened, so the first page stays small
// (coding-standards section 6).
const MyAgentsScreen = lazyRouteComponent(
  () => import("../features/agents/index.ts"),
  "MyAgentsPage",
);
const AgentScreen = lazyRouteComponent(() => import("../features/agents/index.ts"), "AgentPage");
const AgentsSettingsScreen = lazyRouteComponent(
  () => import("../features/settings/index.ts"),
  "AgentsPage",
);

/** *My agents*, `/me/agents`: what waits for the person, each agent with its work, and the standing rules. */
export const myAgentsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.myAgents,
  loader: loadsScreen(MyAgentsScreen),
  component: MyAgentsScreen,
});

/** The page of an agent, `/me/agents/<ID>`: its keys, limits, work and standing rules. */
export const agentRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.agent,
  loader: loadsScreen(AgentScreen),
  component: function Agent() {
    const id = useAddressPart(routeParam.agent);
    return id === undefined ? null : <AgentScreen id={id} />;
  },
});

/** *Organization settings › Connections › Agents*: every agent of the organization, and the settings of agents. */
export const organizationAgentsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.organizationAgents,
  loader: loadsScreen(AgentsSettingsScreen),
  component: function OrganizationAgents() {
    return (
      <OnlyWith path={paths.organizationAgents}>
        <AgentsSettingsScreen />
      </OnlyWith>
    );
  },
});
