/**
 * Whether a link that an agent reported is a web address a person may follow. The agent writes it
 * and the server keeps it as text, so only `http` and `https` are links; anything else is shown as
 * text and never followed.
 */
export function isWebAddress(link: string): boolean {
  try {
    const { protocol } = new URL(link);
    return protocol === "https:" || protocol === "http:";
  } catch {
    return false;
  }
}
