import { t } from "@bugaboo/messages";
import { Button, Icon, IconButton, iconName, useLayout } from "@bugaboo/ui";

import { openOverlay } from "../shell/index.ts";

/** *Assistant* in the top bar: one click away wherever a person is, while AI counts there. */
export function AssistantButton() {
  // On a phone the bar has room for the icon only; the name stays for screen readers.
  if (useLayout() === "phone") {
    return (
      <IconButton
        icon={iconName.assistant}
        label={t("assistant.button")}
        onPress={() => {
          openOverlay("assistant");
        }}
      />
    );
  }
  return (
    <Button
      onPress={() => {
        openOverlay("assistant");
      }}
      aria-label={t("assistant.button")}
    >
      <Icon name={iconName.assistant} />
      {t("assistant.button")}
    </Button>
  );
}
