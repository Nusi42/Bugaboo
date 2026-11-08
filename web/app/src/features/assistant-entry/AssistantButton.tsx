import { t } from "@bugaboo/messages";
import { Button, Icon, IconButton, iconName, useLayout } from "@bugaboo/ui";

import { openAssistantPanel, useAssistantDocked } from "../shell/index.ts";

/** *Assistant* in the top bar: one click away wherever a person is, while AI counts there. */
export function AssistantButton() {
  // Docked beside the page, the panel is open and the button that opened it shows it pressed.
  const docked = useAssistantDocked();
  // On a phone the bar has room for the icon only; the name stays for screen readers.
  if (useLayout() === "phone") {
    return (
      <IconButton
        icon={iconName.assistant}
        label={t("assistant.button")}
        onPress={() => {
          openAssistantPanel();
        }}
      />
    );
  }
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
