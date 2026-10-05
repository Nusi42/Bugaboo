import type { ReactNode } from "react";
import { Button as AriaButton } from "react-aria-components";

import styles from "./Button.module.css";
import { cx } from "./cx.ts";

/** Words of the HTML standard, not text for people. */
const buttonTypes = { button: "button", submit: "submit" } as const;
const currentItem = "true";

export interface ButtonProps {
  readonly children: ReactNode;
  /** The one button of a screen that does what the screen is for. */
  readonly primary?: boolean;
  /** A button that is only text until it is pointed at, for small actions. */
  readonly quiet?: boolean;
  /** A button that takes something away: it is drawn in the color of warnings. */
  readonly danger?: boolean;
  /** Fills the width of its container, as on a phone. */
  readonly wide?: boolean;
  /** Sends the form the button is in, instead of only reacting to a press. */
  readonly submit?: boolean;
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
  primary = false,
  quiet = false,
  danger = false,
  wide = false,
  submit = false,
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
      type={submit && !isPending ? buttonTypes.submit : buttonTypes.button}
      aria-busy={isPending}
      {...(current ? { "aria-current": currentItem } : {})}
      className={cx(
        styles.button,
        primary && styles.primary,
        quiet && styles.quiet,
        danger && styles.danger,
        wide && styles.wide,
      )}
    >
      {children}
    </AriaButton>
  );
}
