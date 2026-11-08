import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook, Overlay, overlayEdge, useLayout } from "@bugaboo/ui";

import { closeOverlay } from "../shell/index.ts";
import type { AssistantStatus } from "./assistant.ts";
import { useLookingAt } from "./use-looking-at.ts";
import { PanelContent } from "./PanelContent.tsx";
import { useClosing } from "./use-closing.ts";
import { useConversation } from "./use-conversation.ts";

/**
 * The assistant (UX §4.12): a panel at the side of a computer and a whole screen on a phone. The
 * conversation lives in this panel only: closing it, by its button, Esc or a press outside, ends it,
 * and asks first when there is something to lose (M7-Q6).
 */
export function AssistantPanel({
  status,
  here,
}: {
  readonly status: AssistantStatus;
  /** AI counts at the place the person is at now; when it does not, the panel says so and asks nothing. */
  readonly here: boolean;
}) {
  const layout = useLayout();
  const { items, leaveOut } = useLookingAt();
  const conversation = useConversation(items);
  const { closing, setClosing, close } = useClosing(conversation.turns.length > 0);
  return (
    <>
      <Overlay
        isOpen
        onOpenChange={(open) => {
          if (!open) {
            close();
          }
        }}
        label={t("assistant.title")}
        from={layout === "phone" ? overlayEdge.full : overlayEdge.end}
      >
        <PanelContent
          status={status}
          here={here}
          items={items}
          onLeaveOut={leaveOut}
          conversation={conversation}
          onClose={close}
        />
      </Overlay>
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
          closeOverlay();
        }}
      />
    </>
  );
}
