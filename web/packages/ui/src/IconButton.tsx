import { Button as AriaButton } from "react-aria-components";

import { Icon } from "./Icon.tsx";
import type { IconName } from "./icons.ts";
import { partOf } from "./parts.ts";
import { buttonLook, buttonShape } from "./props.ts";
import { part } from "./tokens.ts";

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
      {...partOf(part.button, { variants: [buttonLook.quiet, buttonShape.square] })}
      aria-label={label}
      {...(isExpanded === undefined ? {} : { "aria-expanded": isExpanded })}
      onPress={onPress}
    >
      <span title={label} {...partOf(part.iconButtonHit)}>
        <Icon name={icon} />
      </span>
    </AriaButton>
  );
}
