import { type Page } from "@playwright/test";

/**
 * Puts a virtual device with a fingerprint reader into the browser of `page`: passkeys can be made
 * and used without a person touching anything. It lives as long as the browser context does, so
 * the passkeys made with it can be used again later in the same test. (Chromium only.)
 */
export async function addVirtualAuthenticator(page: Page): Promise<void> {
  const session = await page.context().newCDPSession(page);
  await session.send("WebAuthn.enable");
  await session.send("WebAuthn.addVirtualAuthenticator", {
    options: {
      protocol: "ctap2",
      transport: "internal",
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
}
