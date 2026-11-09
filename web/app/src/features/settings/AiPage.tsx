import { partOf, part } from "@bugaboo/ui";

import { QueryGate } from "../shell/index.ts";
import { useAiSetup } from "./ai.ts";
import { AiIntro } from "./AiIntro.tsx";
import { AiAppsCard } from "./AiAppsCard.tsx";
import { AssistantCards } from "./AssistantCards.tsx";

/**
 * *Organization settings › Connections › AI* (AI-01, AI-02, PRIV-08, COMP-08, AI-22): the AI apps
 * that may sign in, then *Set up the assistant* with everything that is only about the assistant.
 * Agents, AI apps and the assistant are each their own switch, in this order (agents are on their
 * own page); the assistant stays off until it is switched on here.
 */
export function AiPage() {
  const setup = useAiSetup();
  return (
    <div {...partOf(part.aiPage)}>
      <AiIntro />
      <AiAppsCard />
      <QueryGate error={setup.error} data={setup.data} loading={null}>
        {(shown) => <AssistantCards setup={shown} />}
      </QueryGate>
    </div>
  );
}
