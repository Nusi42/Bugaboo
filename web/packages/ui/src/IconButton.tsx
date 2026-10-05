import { Button as AriaButton } from "react-aria-components";

import styles from "./Button.module.css";
import { cx } from "./cx.ts";
import { Icon } from "./Icon.tsx";
import type { IconName } from "./icons.ts";
import iconStyles from "./IconButton.module.css";

export interface IconButtonProps {
  readonly icon: IconName;
  /** What the button does, in words: its name for screen readers and its tooltip. */
  readonly label: string;
  readonly onPress: () => void;
  /** Whether a panel or menu that this button opens is open now. */
  readonly isExpanded?: boolean;
}

/** A button with only an icon. For the few icons everybody knows: close, search, menu. */
export function IconButton({ icon, label, onPress, isExpanded }: IconButtonProps) {
  return (
    <AriaButton
      className={cx(styles.button, styles.quiet, iconStyles.square)}
      aria-label={label}
      {...(isExpanded === undefined ? {} : { "aria-expanded": isExpanded })}
      onPress={onPress}
    >
      <span title={label} className={iconStyles.hit}>
        <Icon name={icon} />
      </span>
    </AriaButton>
  );
}
