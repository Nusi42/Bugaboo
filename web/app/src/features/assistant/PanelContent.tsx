import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Icon,
  IconButton,
  iconName,
  partOf,
  part,
  useLayout,
} from "@bugaboo/ui";

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
  // On a phone what the panel looks at and where it answers from scroll with the turns, so the turns keep the screen.
  const phone = useLayout() === "phone";
  const top = (
    <div {...partOf(part.assistantTop)}>
      <LookingAt items={items} onLeaveOut={onLeaveOut} />
      <WhereFrom status={status} />
      <NeverSent />
    </div>
  );
  return (
    <div {...partOf(part.assistantPanel)}>
      <div {...partOf(part.sidePanelHead)}>
        {phone ? (
          // A whole screen of a phone is left with Back, like every other screen of the phone (UX §6.3).
          <Button look={buttonLook.quiet} onPress={onClose}>
            <Icon name={iconName.back} />
            {t("shell.back")}
          </Button>
        ) : null}
        <h2 {...partOf(part.sidePanelTitle)}>{t("assistant.title")}</h2>
        {phone ? null : (
          <IconButton icon={iconName.close} label={t("assistant.close")} onPress={onClose} />
        )}
      </div>
      {phone ? null : top}
      {here ? null : <Alert tone={alertTone.warning}>{t("assistant.off-here")}</Alert>}
      <div {...partOf(part.assistantBody)}>
        {phone ? top : null}
        <TurnList turns={conversation.turns} pending={conversation.pending} />
      </div>
      <AskBox
        isDisabled={!here}
        isPending={conversation.pending !== undefined}
        error={conversation.error}
        answeredCount={conversation.turns.length}
        onAsk={conversation.ask}
      />
      {phone ? <p {...partOf(part.muted)}>{t("assistant.close-text")}</p> : null}
    </div>
  );
}
