import { t } from "@bugaboo/messages";
import { AskDialog } from "@bugaboo/ui";
import { useState } from "react";

import { useHandOver } from "./agent-work.ts";
import {
  emptyDraft,
  handOverBody,
  numbersAreRight,
  type HandOverDraft,
} from "./hand-over-draft.ts";
import { HandOverFields } from "./HandOverFields.tsx";

export interface HandOverDialogProps {
  /** The issues to hand over, by their readable IDs or internal IDs. */
  readonly issues: readonly string[];
  /** What the window is about in its title: the readable ID of one issue; none for several. */
  readonly issue?: string;
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Called once the issues were handed over. */
  readonly onHandedOver?: () => void;
  /** Called with what refused the hand-over; the window is gone by then, so the place it was opened from says it. */
  readonly onFailed?: (error: unknown) => void;
}

/**
 * The window of a hand-over (UX §4.17, §6.3), for one issue and for ticked ones: the agent, the brief
 * with what it lacks named, the ticks of what it may do, the budget, *Plan needs approval* and the end.
 * *Hand over* stays possible with something missing.
 */
export function HandOverDialog(props: HandOverDialogProps) {
  const { issues, issue, isOpen, onOpenChange, onHandedOver, onFailed } = props;
  const [draft, setDraft] = useState<HandOverDraft>(emptyDraft);
  const send = useHandOver(onHandedOver, onFailed);
  return (
    <AskDialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={
        issue === undefined
          ? t("agent-work.hand-over-title-many", { count: issues.length })
          : t("agent-work.hand-over-title", { issue })
      }
      text={t("agent-work.hand-over-text")}
      confirmLabel={t("agent-work.hand-over-confirm")}
      cancelLabel={t("common.cancel")}
      confirmDisabled={draft.agent === "" || !numbersAreRight(draft) || send.isPending}
      onConfirm={() => {
        send.mutate({ body: handOverBody(draft, issues) });
      }}
    >
      <HandOverFields
        draft={draft}
        issues={issues}
        onChange={(next) => {
          setDraft((was) => ({ ...was, ...next }));
        }}
      />
    </AskDialog>
  );
}
