import { t } from "@bugaboo/messages";
import { Button, buttonLook, Icon, iconName } from "@bugaboo/ui";

export interface FocusBarProps {
  readonly onBack: () => void;
  readonly onStep: (direction: 1 | -1) => void;
  /** Whether *Hide list* and *Show list* are offered, and what the list is doing. */
  readonly list: "none" | "shown" | "hidden";
  readonly onToggleList: () => void;
}

/** The buttons above the open item: back, previous, next, and hiding the list. */
export function FocusBar({ onBack, onStep, list, onToggleList }: FocusBarProps) {
  return (
    <>
      <Button look={buttonLook.quiet} onPress={onBack}>
        <Icon name={iconName.back} />
        {t("focus.back")}
      </Button>
      <Button
        onPress={() => {
          onStep(-1);
        }}
      >
        {t("list.previous")}
      </Button>
      <Button
        onPress={() => {
          onStep(1);
        }}
      >
        {t("list.next")}
      </Button>
      {list === "none" ? null : (
        <Button look={buttonLook.quiet} onPress={onToggleList}>
          {list === "hidden" ? t("focus.show-list") : t("focus.hide-list")}
        </Button>
      )}
    </>
  );
}
