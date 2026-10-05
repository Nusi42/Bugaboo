/** Joins the class names that are there. A name that is missing or false is left out. */
export function cx(...names: readonly (string | false | undefined)[]): string {
  return names.filter(Boolean).join(" ");
}
