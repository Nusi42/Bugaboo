import { t } from "@bugaboo/messages";
import { Button, Icon, iconName } from "@bugaboo/ui";

import { openOverlay } from "../shell/index.ts";

/** *Assistant* in the top bar: one click away wherever a person is, while AI counts there. */
export function AssistantButton() {
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
