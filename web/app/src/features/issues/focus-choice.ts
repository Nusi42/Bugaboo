/** Moves the focus to the list of words (the status, the priority) inside a box of a fact. */
export function focusChoice(box: Readonly<Pick<ParentNode, "querySelector">> | null): void {
  box?.querySelector("select")?.focus();
}
