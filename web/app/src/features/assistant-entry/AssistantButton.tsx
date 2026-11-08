import { t } from "@bugaboo/messages";
import { Button, Icon, iconName } from "@bugaboo/ui";

import { openAssistantPanel, useAssistantDocked } from "../shell/index.ts";

/** *Assistant* in the top bar: one click away wherever a person is, while AI counts there. */
export function AssistantButton() {
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
