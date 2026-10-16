/** The ID of the big status button: the key `s` and the bar of a phone open its list. */
export const statusId = "issue-status";

/** Opens the list of the big status button, as a press on it does; nothing when the page has none or it cannot change. */
export function openStatusMenu(): void {
  document.querySelector<HTMLElement>(`#${statusId}`)?.click();
}
