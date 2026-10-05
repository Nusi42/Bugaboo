/** What the form does to the letters typed for a short name: capitals, as the server wants them. */
export function asShortName(typed: string): string {
  return typed.toUpperCase();
}
