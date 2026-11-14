import { t } from "@bugaboo/messages";
import { Button, buttonLook, Icon, iconName } from "@bugaboo/ui";

/** Whether the list has an item before and after the open one. */
export interface Steps {
  readonly previous: boolean;
  readonly next: boolean;
}

export interface FocusBarProps {
  /** *Back to list*; none on a phone, whose top bar has *Back* (UX §4.4, L1). */
  readonly onBack: (() => void) | undefined;
  readonly onStep: (direction: 1 | -1) => void;
  /** Where the open item stands in the list: *Previous* is off at the first, *Next* at the last. */
  readonly steps: Steps;
  /** Whether *Hide list* and *Show list* are offered, and what the list is doing. */
  readonly list: "none" | "shown" | "hidden";
  readonly onToggleList: () => void;
}

/** The buttons above the open item: back, previous, next, and hiding the list. */
export function FocusBar({ onBack, onStep, steps, list, onToggleList }: FocusBarProps) {
  return (
    <>
      {onBack === undefined ? null : (
        <Button look={buttonLook.quiet} onPress={onBack}>
          <Icon name={iconName.back} />
          {t("focus.back")}
        </Button>
      )}
      <Button
        isDisabled={!steps.previous}
        onPress={() => {
          onStep(-1);
        }}
      >
        {t("list.previous")}
      </Button>
      <Button
        isDisabled={!steps.next}
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
