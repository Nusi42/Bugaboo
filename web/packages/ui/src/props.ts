/**
 * The words of the props that pick one of several looks or behaviours of a component. A screen
 * writes `tone={alertTone.danger}`, not a flag per look: a component can then not be asked for two
 * looks at once, and a new look is a new word here.
 */

/** What a message to the person is: a hint, good news, a warning or a problem. */
export const alertTone = {
  info: "info",
  success: "success",
  warning: "warning",
  danger: "danger",
} as const;
export type AlertTone = (typeof alertTone)[keyof typeof alertTone];

/** How a button looks: the one that does what the screen is for, text only, a loss, or the usual. */
export const buttonLook = {
  plain: "plain",
  primary: "primary",
  quiet: "quiet",
  danger: "danger",
} as const;
export type ButtonLook = (typeof buttonLook)[keyof typeof buttonLook];

/** Besides its look, a button can fill the width of its container or be square (an icon). */
export const buttonShape = { wide: "wide", square: "square" } as const;

/** What a button or a text field is doing that the theme file styles: waiting for an answer. */
export const busyState = { busy: "busy" } as const;

/** What a button does in a form: only reacts to a press, or sends the form. */
export const buttonType = { button: "button", submit: "submit" } as const;
export type ButtonType = (typeof buttonType)[keyof typeof buttonType];

/** Where something that opens over the page comes from. */
export const overlayEdge = {
  center: "center",
  full: "full",
  bottom: "bottom",
  start: "start",
  top: "top",
} as const;
export type OverlayEdge = (typeof overlayEdge)[keyof typeof overlayEdge];

/** What a text field holds: plain text, an e-mail address, a password, or several lines. */
export const fieldKind = {
  text: "text",
  email: "email",
  password: "password",
  multiline: "multiline",
} as const;
export type FieldKind = (typeof fieldKind)[keyof typeof fieldKind];

/** A text field that holds several lines looks like one box with room for them. */
export const fieldShape = { multiline: "multiline" } as const;

/** How big an icon is: for navigation, or for lists. */
export const iconSize = { normal: "normal", small: "small" } as const;
export type IconSize = (typeof iconSize)[keyof typeof iconSize];

/** How the choices of a picker look: a scrolling list of plain rows, or a column of bordered boxes. */
export const pickerLook = { list: "list", boxed: "boxed" } as const;
export type PickerLook = (typeof pickerLook)[keyof typeof pickerLook];
