/** Names one after the other as people read a list of them: "Ana, Ben, Cem". */
export function listText(names: readonly string[]): string {
  return names.join(", ");
}
