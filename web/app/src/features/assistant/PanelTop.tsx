import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import type { AssistantStatus } from "./assistant.ts";
import { LookingAt } from "./LookingAt.tsx";
import type { LookingAtItem } from "./looking-at.ts";
import { NeverSent } from "./NeverSent.tsx";
import { WhereFrom } from "./WhereFrom.tsx";

export interface PanelTopProps {
  readonly status: AssistantStatus;
  readonly items: readonly LookingAtItem[];
  readonly onLeaveOut: (key: string) => void;
  /**
   * A phone: where the answers come from stands above the panel's body already, so it is not said again
   * here, and what closing the panel does is said here, at the top, where no turn is cut by it.
   */
  readonly sourceAbove: boolean;
}

/**
 * What the panel looks at, where the answers come from and what is never sent to AI, and the way to
 * the guide to AI (COMP-11). On a computer it stands fixed above the turns; on a phone it scrolls
 * with them, and only where the answers come from stays above.
 */
export function PanelTop({ status, items, onLeaveOut, sourceAbove }: PanelTopProps) {
  return (
    <div {...partOf(part.assistantTop)}>
      <LookingAt items={items} onLeaveOut={onLeaveOut} />
      <WhereFrom status={status} source={!sourceAbove} />
      <NeverSent />
      {sourceAbove ? <p {...partOf(part.muted)}>{t("assistant.close-text")}</p> : null}
      <Link href={paths.aiGuide}>{t("assistant.guide-link")}</Link>
    </div>
  );
}
