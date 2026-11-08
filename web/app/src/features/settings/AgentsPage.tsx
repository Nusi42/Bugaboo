import { t } from "@bugaboo/messages";
import { Card, cardLook, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { OrganizationAgents } from "../agents/index.ts";
import { OrganizationTopic } from "./OrganizationTopic.tsx";

/** The settings of agents: every key of the registry that starts with this. */
const topics = "agents.";

/**
 * *Organization settings › Connections › Agents* (AI-30): agents on or off and the most any agent may be given,
 * through the one settings form, and every agent of the organization with *Stop*.
 */
export function AgentsPage() {
  usePageTitle(t("nav.organization-agents"));
  return (
    <div {...partOf(part.stack)}>
      <Card
        heading={t("nav.organization-agents")}
        level={1}
        look={cardLook.pageHead}
        intro={t("agents.org-intro")}
      />
      <OrganizationTopic topics={topics} />
      <Card heading={t("agents.org-list")}>
        <OrganizationAgents />
      </Card>
    </div>
  );
}
