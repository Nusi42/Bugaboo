import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { formatDate } from "../../shared/format.ts";
import type { Work } from "./agent-work.ts";

/** Why the agent gave up, and when a hand-over that still gives work runs out. */
export function WorkFoot({ work }: { readonly work: Work }) {
  return (
    <>
      {work.gave_up === null || work.gave_up === undefined ? null : (
        <p {...partOf(part.agentWorkWords)}>{t("agent-work.gave-up", { reason: work.gave_up })}</p>
      )}
      {work.open ? (
        <p {...partOf(part.muted)}>{t("agent-work.ends", { date: formatDate(work.ends_at) })}</p>
      ) : null}
    </>
  );
}
