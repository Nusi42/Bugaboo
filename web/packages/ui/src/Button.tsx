import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";

import styles from "./Button.module.css";
import { cx } from "./cx.ts";
import { buttonLook, buttonType, type ButtonLook, type ButtonType } from "./props.ts";

/** A word of the ARIA standard, not text for people. */
const currentItem = "true";

export interface ButtonProps {
  readonly children: ReactNode;
  /**
   * `primary` is the one button of a screen that does what the screen is for; `quiet` is only
   * text until it is pointed at, for small actions; `danger` takes something away and is drawn in
   * the color of warnings.
   */
  readonly look?: ButtonLook;
  /** Fills the width of its container, as on a phone. */
  readonly wide?: boolean;
  /** `submit` sends the form the button is in, instead of only reacting to a press. */
  readonly type?: ButtonType;
  /**
   * Waiting for an answer: the button does nothing when pressed again, and says it is busy. (The
   * pending state of React Aria Components is not used: it announces itself with an element that
   * accessibility checkers find without a name.)
   */
  readonly isPending?: boolean;
  /** The button of the item that is open now, in a list of items: marked for screen readers. */
  readonly current?: boolean;
  readonly isDisabled?: boolean;
  readonly onPress?: () => void;
  /** The name for screen readers when the text alone is not enough, for example in a list. */
  readonly "aria-label"?: string;
}

export function Button({
  children,
  look = buttonLook.plain,
  wide = false,
  type = buttonType.button,
  isPending = false,
  current = false,
  onPress,
  ...props
}: ButtonProps) {
  return (
    <AriaButton
      {...props}
      {...(isPending || onPress === undefined ? {} : { onPress })}
      // A button that waits for an answer must not send the form a second time.
      type={type === buttonType.submit && !isPending ? buttonType.submit : buttonType.button}
      aria-busy={isPending}
      {...(current ? { "aria-current": currentItem } : {})}
      className={cx(styles.button, look !== buttonLook.plain && styles[look], wide && styles.wide)}
    >
      {children}
    </AriaButton>
  );
}
