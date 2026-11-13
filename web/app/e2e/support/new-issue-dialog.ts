import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { part } from "../../../packages/ui/src/tokens.ts";
import { partSelector } from "./parts.ts";

/** The name of the box of the title on the form of a new issue: every required field says so. */
export function titleLabel(): string {
  return t("fields.label-required", { field: t("issue-field.title") });
}

/** The dialog that makes an issue. */
export function newIssueDialog(page: Page) {
  return page.getByRole("dialog", { name: t("issues.new-title"), exact: true });
}

/** The button *New issue* of the bar: the sidebar on a computer, the bar of a phone. */
function newIssueButton(page: Page) {
  return page.getByRole("button", { name: t("issues.new"), exact: true }).first();
}

/** What the window was doing, and what its status said, when Esc was pressed. */
export interface AtEscape {
  readonly state: string;
  readonly status: string;
}

/**
 * Presses *New issue* and Esc in the same moment, in the page: Esc is sent from the first `focusin`
 * inside the window, as soon as the window has taken the focus. Says what the window was doing then.
 */
export function pressNewIssueAndEscapeAtOnce(page: Page): Promise<AtEscape> {
  return newIssueButton(page).evaluate(
    (button, box) =>
      new Promise<AtEscape>((done) => {
        const escapeOnFocus = (): void => {
          const modal = document.querySelector<HTMLElement>(box);
          const target = document.activeElement;
          if (modal === null || target === null || !modal.contains(target)) {
            return;
          }
          document.removeEventListener("focusin", escapeOnFocus);
          const seen = {
            state: modal.dataset["state"] ?? "",
            status: modal.querySelector("output")?.textContent ?? "",
          };
          target.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
          done(seen);
        };
        document.addEventListener("focusin", escapeOnFocus);
        button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      }),
    partSelector(part.overlayModal),
  );
}

/**
 * The window was in its clear first 0.3 s and said nothing yet when Esc came. This is asserted every
 * time: the code of the window is loaded before, so Esc follows the click within a few frames.
 * `page.clock` cannot hold the 0.3 s back instead: stopping its timers stops the frames of the page
 * too, and then the window never opens.
 */
export function expectQuietAtEscape(atEscape: AtEscape): void {
  expect(atEscape.state.split(" "), "the window was in its clear first 0.3 s").toContain("quiet");
  expect(atEscape.status, "nothing is said before the grey shapes show").toBe("");
}

/** Presses *New issue* in the bar (the sidebar on a computer, the bar of a phone); the form may still be on its way. */
export async function pressNewIssue(page: Page): Promise<void> {
  await newIssueButton(page).click();
  await expect(newIssueDialog(page)).toBeVisible();
}

/** Waits until the dialog shows its form: until then grey shapes stand in for it. */
export async function waitForNewIssueForm(page: Page): Promise<void> {
  await expect(
    newIssueDialog(page).locator('[data-part="window.body"][aria-busy="false"]'),
  ).toBeVisible();
}

/** Opens the dialog with the button of the bar and waits for its form. */
export async function openNewIssueDialog(page: Page): Promise<void> {
  await pressNewIssue(page);
  await waitForNewIssueForm(page);
}

/** Opens the dialog with the button in the menu of a phone. */
export async function openNewIssueDialogFromTheMenu(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: t("shell.menu") })
    .getByRole("button", { name: t("issues.new"), exact: true })
    .click();
  await waitForNewIssueForm(page);
}
