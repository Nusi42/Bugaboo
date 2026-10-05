import {
  FieldError,
  Input,
  Label,
  Text,
  TextArea,
  TextField as AriaTextField,
} from "react-aria-components";

import { autoComplete, type AutoComplete } from "./autocomplete.ts";
import styles from "./Field.module.css";
import { cx } from "./cx.ts";

/** Words of the HTML standard, not text for people. */
const inputTypes = { text: "text", password: "password" } as const;
const inputModes = { text: "text", email: "email" } as const;

/** The name of the slot of the help text, in the words of React Aria Components. */
const descriptionSlot = "description";

export interface TextFieldProps {
  /** The visible name of the field, already in the person's language. */
  readonly label: string;
  /** Help under the name, for example what the field is for. */
  readonly description?: string;
  /** What to say when the field is empty. Without it the field may be empty. */
  readonly requiredMessage?: string;
  /** What to say when an e-mail address has no `@`. Only for an `email` field. */
  readonly emailMessage?: string;
  /** A problem the server found with the value: shown under the field. */
  readonly errorMessage?: string | undefined;
  /** An e-mail address: the keyboard of a phone offers the @ sign, and the value is checked. */
  readonly email?: boolean;
  /** A password: the characters are hidden. */
  readonly password?: boolean;
  /** More than one line (for lists). */
  readonly multiline?: boolean;
  readonly name?: string;
  readonly value: string;
  readonly onChange: (value: string) => void;
  /** What a password manager or the browser may fill in: one of `autoComplete`. */
  readonly autoComplete?: AutoComplete;
}

/**
 * The browser's own check of an e-mail field speaks the language of the browser, not ours, so
 * the field is checked here and says what the message catalog says. The mobile keyboard for
 * e-mail addresses is asked for with `inputMode`.
 */
function problemOf(
  text: string,
  email: boolean,
  requiredMessage: string | undefined,
  emailMessage: string | undefined,
): string | null {
  if (requiredMessage !== undefined && text.trim() === "") {
    return requiredMessage;
  }
  if (email && emailMessage !== undefined && text.trim() !== "" && !text.includes("@")) {
    return emailMessage;
  }
  return null;
}

/** A label, a text box, help and a problem, tied together for screen readers. */
export function TextField({
  label,
  description,
  requiredMessage,
  emailMessage,
  errorMessage,
  email = false,
  password = false,
  multiline = false,
  name = "",
  value,
  onChange,
  autoComplete: fill = autoComplete.off,
}: TextFieldProps) {
  return (
    <AriaTextField
      className={cx(styles.field)}
      name={name}
      value={value}
      onChange={onChange}
      type={password ? inputTypes.password : inputTypes.text}
      inputMode={email ? inputModes.email : inputModes.text}
      autoComplete={fill}
      // Only a problem is passed on: `isInvalid={false}` would switch off the checks of the field.
      {...(errorMessage === undefined ? {} : { isInvalid: true })}
      validate={(text) => problemOf(text, email, requiredMessage, emailMessage)}
    >
      <Label className={cx(styles.label)}>{label}</Label>
      {description === undefined ? null : (
        <Text slot={descriptionSlot} className={cx(styles.description)}>
          {description}
        </Text>
      )}
      {multiline ? (
        <TextArea className={cx(styles.input, styles.multiline)} rows={4} />
      ) : (
        <Input className={cx(styles.input)} />
      )}
      <FieldError className={cx(styles.error)}>{errorMessage}</FieldError>
    </AriaTextField>
  );
}
