import { t } from "@bugaboo/messages";
import { Alert, alertTone, IconButton, iconName, partOf, part } from "@bugaboo/ui";

import type { AssistantStatus } from "./assistant.ts";
import { AskBox } from "./AskBox.tsx";
import { LookingAt } from "./LookingAt.tsx";
import type { LookingAtItem } from "./looking-at.ts";
import { NeverSent } from "./NeverSent.tsx";
import { TurnList } from "./TurnList.tsx";
import type { Conversation } from "./use-conversation.ts";
import { WhereFrom } from "./WhereFrom.tsx";

export interface PanelContentProps {
  readonly status: AssistantStatus;
  readonly here: boolean;
  readonly items: readonly LookingAtItem[];
  readonly onLeaveOut: (key: string) => void;
  readonly conversation: Conversation;
  readonly onClose: () => void;
}

/**
 * What stands in the panel: its title and close button, on top what it looks at, where its answers
 * come from and what is never sent to AI, then the turns, and at the foot the box to ask.
 */
export function PanelContent({
  status,
  here,
  items,
  onLeaveOut,
  conversation,
  onClose,
}: PanelContentProps) {
  return (
    <div {...partOf(part.assistantPanel)}>
      <div {...partOf(part.sidePanelHead)}>
        <h2 {...partOf(part.sidePanelTitle)}>{t("assistant.title")}</h2>
        <IconButton icon={iconName.close} label={t("assistant.close")} onPress={onClose} />
      </div>
      <div {...partOf(part.assistantTop)}>
        <LookingAt items={items} onLeaveOut={onLeaveOut} />
        <WhereFrom status={status} />
        <NeverSent />
      </div>
      {here ? null : <Alert tone={alertTone.warning}>{t("assistant.off-here")}</Alert>}
      <div {...partOf(part.assistantBody)}>
        <TurnList turns={conversation.turns} pending={conversation.pending} />
      </div>
      <AskBox
        isDisabled={!here}
        isPending={conversation.pending !== undefined}
        error={conversation.error}
        answeredCount={conversation.turns.length}
        onAsk={conversation.ask}
      />
    </div>
  );
}
