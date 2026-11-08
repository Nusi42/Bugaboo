import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { formatDate } from "../../shared/format.ts";
import type { Work } from "./agent-work.ts";
import { HandOverDialog } from "./HandOverDialog.tsx";
import { outcomeOf } from "./outcome.ts";

/** Whether the work ended without a result to accept: the agent gave up, or it was stopped, or a result was thrown away. */
function endedWithoutResult(work: Work): boolean {
  return (
    !work.open &&
    (work.state === "agent-gave-up" ||
      work.state === "stopped" ||
      outcomeOf(work) === "thrown-away")
  );
}

/** Why the work ended, in words: who gave up and why, or who stopped it; nothing for work that went on to a review. */
function WhyEnded({ work }: { readonly work: Work }) {
  if (work.gave_up !== null && work.gave_up !== undefined) {
    return (
      <p {...partOf(part.agentWorkWords)}>
        {t("agent-work.gave-up", { agent: work.agent_name, reason: work.gave_up })}
      </p>
    );
  }
  if (work.state === "stopped" && outcomeOf(work) === undefined && !work.open) {
    return (
      <p {...partOf(part.agentWorkWords)}>
        {t("agent-work.stopped-why", {
          agent: work.agent_name,
          date: formatDate(work.ends_at),
        })}
      </p>
    );
  }
  return null;
}

/** The next step of work that ended without a result: hand the issue over again, or take it back and work on it. */
function NextStep({ work, issue }: { readonly work: Work; readonly issue: string }) {
  const [open, setOpen] = useState(false);
  if (!endedWithoutResult(work)) {
    return null;
  }
  return (
    <div {...partOf(part.agentWorkSection)}>
      <p {...partOf(part.muted)}>{t("agent-work.next-step")}</p>
      <div {...partOf(part.agentWorkActions)}>
        <Button
          onPress={() => {
            setOpen(true);
          }}
        >
          {t("agent-work.hand-over-again")}
        </Button>
      </div>
      {open ? (
        <HandOverDialog issues={[issue]} issue={issue} isOpen onOpenChange={setOpen} />
      ) : null}
    </div>
  );
}

/**
 * The end of a piece of work: why the agent gave up or was stopped and what to do next (only on
 * the newest work of the issue), and when a hand-over that still gives work runs out.
 */
export function WorkFoot({
  work,
  issue,
  isNewest,
}: {
  readonly work: Work;
  readonly issue: string;
  /** Whether it is the newest work of the issue: older work has a newer hand-over after it. */
  readonly isNewest: boolean;
}) {
  return (
    <>
      <WhyEnded work={work} />
      {isNewest ? <NextStep work={work} issue={issue} /> : null}
      {work.open ? (
        <p {...partOf(part.muted)}>{t("agent-work.ends", { date: formatDate(work.ends_at) })}</p>
      ) : null}
    </>
  );
}
