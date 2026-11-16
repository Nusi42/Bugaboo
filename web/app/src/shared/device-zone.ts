/** The time zone of this device, by its name. Kept free of imports, so plain Node tools (the speed driver) can read it too. */
export function deviceTimeZone(): string {
  return new Intl.DateTimeFormat().resolvedOptions().timeZone;
}
