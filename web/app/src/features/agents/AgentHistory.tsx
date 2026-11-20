import { t } from "@bugaboo/messages";
import { Card, IssueId, partOf, part, SentenceWithValues, ShowMoreOf } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { itemsOf } from "../../shared/paged.ts";
import { deedWords, useAgentHistory, type AgentDeed } from "./agent-history.ts";
import type { ListedAgent } from "./agents.ts";

/** One entry: when, and what the agent did, as the entries of an issue's history look. */
function DeedEntry({ deed }: { readonly deed: AgentDeed }) {
  const words = deedWords(deed);
  return (
    <article {...partOf(part.historyEntry)}>
      <span {...partOf(part.historyWhen)}>{formatDateTime(deed.at)}</span>
      <span {...partOf(part.itemText)}>
        <SentenceWithValues sentence={words.sentence} values={words.issues}>
          {(id) => <IssueId id={id} />}
        </SentenceWithValues>
      </span>
    </article>
  );
}

/** *History* of an agent (AI-37): what it handed over to the person's other agents and which of its runs it undid, newest first. */
export function AgentHistory({ agent }: { readonly agent: ListedAgent }) {
  const found = useAgentHistory(agent.id);
  const deeds = itemsOf(found.data?.pages);
  return (
    <Card heading={t("history.title")}>
      {deeds.length === 0 && found.isSuccess && !found.hasNextPage ? (
        <p {...partOf(part.muted)}>{t("agents.history-none")}</p>
      ) : null}
      <div {...partOf(part.stack)}>
        {deeds.map((deed: AgentDeed) => (
          <DeedEntry key={deed.id} deed={deed} />
        ))}
      </div>
      <ShowMoreOf label={t("history.more")} list={found} />
    </Card>
  );
}
