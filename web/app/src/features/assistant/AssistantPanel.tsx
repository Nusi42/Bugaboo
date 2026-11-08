import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook } from "@bugaboo/ui";

import { closeAssistantPanel } from "../shell/index.ts";
import type { AssistantStatus } from "./assistant.ts";
import { PanelContent } from "./PanelContent.tsx";
import { PanelShell } from "./PanelShell.tsx";
import { useClosing } from "./use-closing.ts";
import { useConversation } from "./use-conversation.ts";
import { useLookingAt } from "./use-looking-at.ts";

/**
 * The assistant (UX §4.12): a panel beside the page on a computer, which the page does not give way
 * to dimming, a layer from the end on a tablet and a whole screen on a phone. The conversation lives in
 * this panel only: closing it, by its button or Esc, ends it, and asks first when there is something
 * to lose (M7-Q6).
 */
export function AssistantPanel({
  status,
  here,
}: {
  readonly status: AssistantStatus;
  /** AI counts at the place the person is at now; when it does not, the panel says so and asks nothing. */
  readonly here: boolean;
}) {
  const { items, leaveOut } = useLookingAt();
  const conversation = useConversation(items);
  const { closing, setClosing, close } = useClosing(conversation.turns.length > 0);
  const content = (
    <PanelContent
      status={status}
      here={here}
      items={items}
      onLeaveOut={leaveOut}
      conversation={conversation}
      onClose={close}
    />
  );
  return (
    <>
      <PanelShell onClose={close}>{content}</PanelShell>
      <AskDialog
        isOpen={closing}
        onOpenChange={setClosing}
        title={t("assistant.close-title")}
        text={t("assistant.close-text")}
        confirmLabel={t("assistant.close-yes")}
        cancelLabel={t("assistant.close-keep")}
        confirmLook={buttonLook.primary}
        onConfirm={() => {
          setClosing(false);
          closeAssistantPanel();
        }}
      />
    </>
  );
}
