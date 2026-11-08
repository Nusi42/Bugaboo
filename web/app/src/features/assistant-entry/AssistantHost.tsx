import { t } from "@bugaboo/messages";
import { useLocation } from "@tanstack/react-router";
import { lazy, Suspense, useEffect, useState } from "react";

import { projectOf } from "../../shared/assistant-place.ts";
import {
  addQuickAction,
  addTopBarAction,
  openAssistantPanel,
  useAssistantPanelOpen,
  type TopBarAction,
} from "../shell/index.ts";
import { AssistantButton } from "./AssistantButton.tsx";
import { useAssistantStatus, type EntryStatus } from "./status.ts";

/** The panel and its texts are loaded when it is first opened, so the first page does not grow beyond the button (ADR-0028). */
const AssistantPanel = lazy(async () => ({
  default: (await import("../assistant/index.ts")).AssistantPanel,
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
    openAssistantPanel();
  },
};

/**
 * The assistant for the shell: while AI counts at the place it puts the button into the top bar and an
 * entry into quick search, and draws the panel when it is open. Where AI is off, for the organization
 * or the project, nothing of it is there (AI-02). A panel that is open stays open when the person goes
 * to a place where AI is off, and says so: the conversation is never dropped without asking (M7-Q6).
 */
export function AssistantHost() {
  const location = useLocation();
  const status = useAssistantStatus(projectOf(location.pathname, location.search));
  const available = status?.available === true;
  const open = useAssistantPanelOpen();
  const [kept, setKept] = useState<EntryStatus>();
  if (available && status !== kept) {
    setKept(status);
  }
  useEffect(() => {
    const gone = available ? [addTopBarAction(topBarButton), addQuickAction(quickAsk)] : [];
    return () => {
      for (const take of gone) {
        take();
      }
    };
  }, [available]);
  const shown = available ? status : kept;
  return open && shown !== undefined ? (
    <Suspense fallback={null}>
      <AssistantPanel status={shown} here={available} />
    </Suspense>
  ) : null;
}
