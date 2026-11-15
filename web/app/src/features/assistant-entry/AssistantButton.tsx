import { t } from "@bugaboo/messages";
import { Button, Icon, iconName, KeptRoom } from "@bugaboo/ui";
import { useLocation } from "@tanstack/react-router";

import { projectOf } from "../../shared/assistant-place.ts";
import { openAssistantPanel, useAssistantDocked } from "../shell/index.ts";
import { useAssistantStatusRead } from "./status.ts";

/**
 * The place of *Assistant* in the top bar: the button while AI counts at the place, nothing where it does not. Until the
 * first answer the button keeps its room, unseen, so it does not come in beside what is drawn and narrow it; where AI
 * does not count, the room goes and what stands beside it only grows (UX §6.4).
 */
export function AssistantTopBarButton() {
  const location = useLocation();
  const read = useAssistantStatusRead(projectOf(location.pathname, location.search));
  if (read.isLoading) {
    return (
      <KeptRoom>
        <AssistantButton />
      </KeptRoom>
    );
  }
  return read.data?.available === true ? <AssistantButton /> : null;
}

/** *Assistant* in the top bar: one click away wherever a person is, while AI counts there. */
function AssistantButton() {
  // Docked beside the page, the panel is open and the button that opened it shows it pressed.
  const docked = useAssistantDocked();
  return (
    <Button
      current={docked}
      onPress={() => {
        openAssistantPanel();
      }}
      aria-label={t("assistant.button")}
    >
      <Icon name={iconName.assistant} />
      {t("assistant.button")}
    </Button>
  );
}
