import { t } from "@bugaboo/messages";
import { Button, IssueId, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { issueAddress } from "../../shared/paths.ts";
import { type MyWorkItem } from "./agents.ts";
import { Section } from "./Section.tsx";

/** How many checks the last attempt of the work passed. */
function checksPassed(item: MyWorkItem): number {
  const attempt = item.work.attempts.at(-1);
  return (
    attempt?.results.filter((result) => result.kind === "check" && result.outcome === "passed")
      .length ?? 0
  );
}

/**
 * *Accepted by itself* (UX §4.17): attempts that were accepted because every check passed, until a
 * person looked at them. *Open* goes to the issue, where looking at the work ends the line.
 */
export function AcceptedByItself({ work }: { readonly work: readonly MyWorkItem[] }) {
  const navigate = useNavigate();
  const accepted = work.filter((item) => item.work.accepted_by_itself);
  if (accepted.length === 0) {
    return null;
  }
  return (
    <Section heading={t("agents.accepted")} count={accepted.length}>
      <ul {...partOf(part.list)}>
        {accepted.map((item) => (
          <li key={item.work.id} {...partOf(part.item)}>
            <span {...partOf(part.startColumn)}>
              <span {...partOf(part.itemTitle)}>
                <IssueId id={item.issue.readable_id} /> {item.issue.title ?? ""}
              </span>
              <span {...partOf(part.muted)}>
                {t("agents.accepted-line", {
                  agent: item.work.agent_name,
                  count: checksPassed(item),
                })}
              </span>
            </span>
            <Button
              aria-label={t("agents.open-name", { issue: item.issue.readable_id })}
              onPress={() => {
                void navigate({ href: issueAddress(item.issue.readable_id) });
              }}
            >
              {t("agents.open")}
            </Button>
          </li>
        ))}
      </ul>
    </Section>
  );
}
