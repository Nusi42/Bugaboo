import { t } from "@bugaboo/messages";
import { lazy, Suspense, useEffect } from "react";
import { useLocation } from "@tanstack/react-router";

import {
  addQuickAction,
  addTopBarAction,
  openOverlay,
  useOverlayOpen,
  type TopBarAction,
} from "../shell/index.ts";
import { useAssistantStatus } from "./assistant.ts";
import { AssistantButton } from "./AssistantButton.tsx";
import { projectOf } from "./looking-at.ts";

/** The panel is loaded when it is first opened, so the first page does not grow beyond the button (ADR-0028). */
const AssistantPanel = lazy(async () => ({
  default: (await import("./AssistantPanel.tsx")).AssistantPanel,
}));

/** The button at the end of the top bar. */
const topBarButton: TopBarAction = { id: "assistant", render: () => <AssistantButton /> };

/** The entry of quick search that opens the panel. */
const quickAsk = {
  id: "ask-assistant",
  get title() {
    return t("assistant.quick-ask");
  },
  run: () => {
    openOverlay("assistant");
  },
};

/**
 * The assistant for the shell: while AI counts at the place it puts the button into the top bar and an
 * entry into quick search, and draws the panel when it is open. Where AI is off, for the organization
 * or the project, nothing of it is there (AI-02).
 */
export function AssistantHost() {
  const location = useLocation();
  const status = useAssistantStatus(projectOf(location.pathname, location.search));
  const available = status?.available === true;
  const open = useOverlayOpen("assistant");
  useEffect(() => {
    const gone = available ? [addTopBarAction(topBarButton), addQuickAction(quickAsk)] : [];
    return () => {
      for (const take of gone) {
        take();
      }
    };
  }, [available]);
  return open && available ? (
    <Suspense fallback={null}>
      <AssistantPanel status={status} />
    </Suspense>
  ) : null;
}
