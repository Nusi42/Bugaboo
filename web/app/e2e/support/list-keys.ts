import { expect, type Locator, type Page, type Request } from "@playwright/test";

/** The keys of the list are for a person who is not typing: the focus leaves whatever box the page began in. */
export async function leaveBox(page: Page): Promise<void> {
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });
}

/** Ticks or unticks a box by pressing its label, as a person does (the input itself is not what is drawn). */
export async function setBox(box: Locator, on: boolean): Promise<void> {
  if ((await box.isChecked()) !== on) {
    await box.locator("xpath=ancestor::label").click();
  }
  await (on ? expect(box).toBeChecked() : expect(box).not.toBeChecked());
}

/** The addresses the page asks for from now on, written out (the text of a search is in them as the person wrote it). */
export function recordRequests(page: Page): string[] {
  const asked: string[] = [];
  page.on("request", (request: Readonly<Pick<Request, "url">>) => {
    asked.push(decodeURIComponent(request.url()).replaceAll("+", " "));
  });
  return asked;
}
