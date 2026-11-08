import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook, Overlay, overlayEdge, useLayout } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";
import { useState } from "react";

import { closeOverlay } from "../shell/index.ts";
import type { AssistantStatus } from "./assistant.ts";
import { lookingAtOf, withoutItem } from "./looking-at.ts";
import { PanelContent } from "./PanelContent.tsx";
import { useClosing } from "./use-closing.ts";
import { useConversation } from "./use-conversation.ts";

/**
 * The assistant (UX §4.12): a panel at the side of a computer and a whole screen on a phone. The
 * conversation lives in this panel only: closing it, by its button, Esc or a press outside, ends it,
 * and asks first when there is something to lose (M7-Q6).
 */
export function AssistantPanel({ status }: { readonly status: AssistantStatus }) {
  const layout = useLayout();
  const location = useLocation();
  const [items, setItems] = useState(() => lookingAtOf(location.pathname, location.search));
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
          items={items}
          onLeaveOut={(key) => {
            setItems((before) => withoutItem(before, key));
          }}
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
