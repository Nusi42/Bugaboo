import {
  FieldError,
  Input,
  Label,
  Text,
  TextArea,
  TextField as AriaTextField,
} from "react-aria-components";

import { autoComplete, type AutoComplete } from "./autocomplete.ts";
import { partOf } from "./parts.ts";
import { fieldKind, fieldShape, type FieldKind } from "./props.ts";
import { part } from "./tokens.ts";

/** Words of the HTML standard, not text for people. */
const inputTypes = { text: "text", password: "password", date: "date" } as const;
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
  /**
   * The name is there for screen readers and voice control only, for a field that stands in a row
   * that already names it (a row of the Details). Never the way to save room when the name matters
   * to everybody.
   */
  readonly hideLabel?: boolean;
  /**
   * What the field holds. An `email` address: the keyboard of a phone offers the @ sign, and the
   * value is checked. A `password`: the characters are hidden. `multiline`: more than one line (for
   * lists). Plain `text` is the default.
   */
  readonly kind?: FieldKind;
  /** A day: the browser offers its own calendar. `min` and `max` are days as `2027-03-01`. */
  readonly date?: { readonly min: string; readonly max: string };
  readonly name?: string;
  readonly value: string;
  readonly onChange: (value: string) => void;
  /** What a password manager or the browser may fill in: one of `autoComplete`. */
  readonly autoComplete?: AutoComplete;
}

/** The kind of input the browser draws: a day, hidden characters, or plain text. */
function typeOf(kind: FieldKind, day: boolean): (typeof inputTypes)[keyof typeof inputTypes] {
  if (day) {
    return inputTypes.date;
  }
  return kind === fieldKind.password ? inputTypes.password : inputTypes.text;
}

/**
 * The browser's own check of an e-mail field speaks the language of the browser, not ours, so
 * the field is checked here and says what the message catalog says. The mobile keyboard for
 * e-mail addresses is asked for with `inputMode`.
 */
function problemOf(
  text: string,
  kind: FieldKind,
  requiredMessage: string | undefined,
  emailMessage: string | undefined,
): string | null {
  if (requiredMessage !== undefined && text.trim() === "") {
    return requiredMessage;
  }
  if (
    kind === fieldKind.email &&
    emailMessage !== undefined &&
    text.trim() !== "" &&
    !text.includes("@")
  ) {
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
  hideLabel = false,
  kind = fieldKind.text,
  date,
  name = "",
  value,
  onChange,
  autoComplete: fill = autoComplete.off,
}: TextFieldProps) {
  return (
    <AriaTextField
      {...partOf(part.field)}
      name={name}
      value={value}
      onChange={onChange}
      type={typeOf(kind, date !== undefined)}
      inputMode={kind === fieldKind.email ? inputModes.email : inputModes.text}
      autoComplete={fill}
      // Only a problem is passed on: `isInvalid={false}` would switch off the checks of the field.
      {...(errorMessage === undefined ? {} : { isInvalid: true })}
      validate={(text) => problemOf(text, kind, requiredMessage, emailMessage)}
    >
      <Label {...partOf(hideLabel ? part.visuallyHidden : part.fieldLabel)}>{label}</Label>
      {description === undefined ? null : (
        <Text slot={descriptionSlot} {...partOf(part.fieldDescription)}>
          {description}
        </Text>
      )}
      {kind === fieldKind.multiline ? (
        <TextArea {...partOf(part.fieldInput, { variants: [fieldShape.multiline] })} rows={4} />
      ) : (
        <Input
          {...partOf(part.fieldInput)}
          {...(date === undefined ? {} : { min: date.min, max: date.max })}
        />
      )}
      <FieldError {...partOf(part.fieldError)}>{errorMessage}</FieldError>
    </AriaTextField>
  );
}
