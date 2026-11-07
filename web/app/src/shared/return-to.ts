import { paths } from "./paths.ts";

/** The key under which the tab keeps the address of the consent step while the person signs in. */
const KEY = "bugaboo.return-to";

/** Whether an address is one the person may be sent back to: the consent step of an AI app and nothing else. */
export function isReturnAddress(address: string): boolean {
  return address === paths.connect || address.startsWith(`${paths.connect}?`);
}

/**
 * Remembers the page a signed-out person asked for, so that after signing in they get there. Only
 * the consent step of an AI app is remembered (AI-22): an app sends the person to it, and signing in
 * must not lose it. Any other address is forgotten, so the memory cannot be used to send a person
 * somewhere else.
 */
export function rememberReturn(address: string): void {
  try {
    if (isReturnAddress(address)) {
      sessionStorage.setItem(KEY, address);
    }
  } catch {
    // A tab that cannot keep it sends the person to the start page, as before.
  }
}

/** The page to go to after signing in, once: none when nothing was remembered. */
export function takeReturn(): string | null {
  try {
    const address = sessionStorage.getItem(KEY);
    sessionStorage.removeItem(KEY);
    return address !== null && isReturnAddress(address) ? address : null;
  } catch {
    return null;
  }
}
