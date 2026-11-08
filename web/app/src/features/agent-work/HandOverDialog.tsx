import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part, Window } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { windowLabels } from "../../shared/window-labels.ts";
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
  /** Who the issue stays assigned to, in words, when one issue is handed over and has people. */
  readonly assigned?: string | undefined;
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Called once the issues were handed over. */
  readonly onHandedOver?: () => void;
}

/** The buttons of the window: *Cancel*, and *Hand over*, which waits for an agent and for numbers that are numbers. */
function HandOverFooter({
  draft,
  isPending,
  onSend,
  onClose,
}: {
  readonly draft: HandOverDraft;
  readonly isPending: boolean;
  readonly onSend: () => void;
  readonly onClose: () => void;
}) {
  return (
    <div {...partOf(part.windowFooterActions)}>
      <Button look={buttonLook.quiet} onPress={onClose}>
        {t("common.cancel")}
      </Button>
      <Button
        look={buttonLook.primary}
        isPending={isPending}
        isDisabled={draft.agent === "" || !numbersAreRight(draft)}
        onPress={onSend}
      >
        {t("agent-work.hand-over-confirm")}
      </Button>
    </div>
  );
}

/**
 * The window of a hand-over (UX §4.17, §6.3), for one issue and for ticked ones, in the one window
 * part of the app: a window on a computer, a full screen with *Back* on a phone. It holds the agent,
 * the brief with what it lacks named, the ticks of what it may do, the budget, *Plan needs approval*
 * and the end. *Hand over* stays possible with something missing; a refusal is said in the window.
 */
export function HandOverDialog(props: HandOverDialogProps) {
  const { issues, issue, assigned, isOpen, onOpenChange, onHandedOver } = props;
  const [draft, setDraft] = useState<HandOverDraft>(emptyDraft);
  const send = useHandOver(onHandedOver);
  return (
    <Window
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      labels={windowLabels()}
      title={
        issue === undefined
          ? t("agent-work.hand-over-title-many", { count: issues.length })
          : t("agent-work.hand-over-title", { issue })
      }
      footer={
        <HandOverFooter
          draft={draft}
          isPending={send.isPending}
          onSend={() => {
            send.mutate(
              { body: handOverBody(draft, issues) },
              {
                onSuccess: () => {
                  onOpenChange(false);
                },
              },
            );
          }}
          onClose={() => {
            onOpenChange(false);
          }}
        />
      }
    >
      <div {...partOf(part.agentWorkWindow)}>
        <HandOverFields
          draft={draft}
          issues={issues}
          assigned={assigned}
          onChange={(next) => {
            setDraft((was) => ({ ...was, ...next }));
          }}
        />
        {send.isError ? <Alert tone={alertTone.danger}>{messageOf(send.error)}</Alert> : null}
      </div>
    </Window>
  );
}
