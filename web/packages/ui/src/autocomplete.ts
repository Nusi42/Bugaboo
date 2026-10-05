/**
 * What a browser or a password manager may fill in, in the words of the HTML standard. Written
 * here once, so a screen names its field with `autoComplete.username` and no literal text.
 */
export const autoComplete = {
  username: "username",
  currentPassword: "current-password",
  newPassword: "new-password",
  name: "name",
  oneTimeCode: "one-time-code",
  email: "email",
  off: "off",
} as const;

/** One of the words of {@link autoComplete}. */
export type AutoComplete = (typeof autoComplete)[keyof typeof autoComplete];
