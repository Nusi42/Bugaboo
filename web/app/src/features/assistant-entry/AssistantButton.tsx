import { t } from "@bugaboo/messages";
import { Button, Icon, IconButton, iconName, useLayout } from "@bugaboo/ui";

import { openAssistantPanel } from "../shell/index.ts";

/** *Assistant* in the top bar: one click away wherever a person is, while AI counts there. */
export function AssistantButton() {
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
