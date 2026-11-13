import { t } from "@bugaboo/messages";
import { expect, test, type Page } from "@playwright/test";

import { skeletonDelayMs } from "../../../packages/ui/src/after-delay.ts";
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

/** What the window was doing, what its status said, and how long after the click, when Esc was pressed. */
export interface AtEscape {
  readonly state: string;
  readonly status: string;
  readonly elapsedMs: number;
}

/**
 * Presses *New issue* and Esc in the same moment, in the page: Esc is sent from the first `focusin`
 * inside the window, as soon as the window has taken the focus. Says what the window was doing then.
 */
export function pressNewIssueAndEscapeAtOnce(page: Page): Promise<AtEscape> {
  return newIssueButton(page).evaluate(
    (button, box) =>
      new Promise<AtEscape>((done) => {
        const clicked = performance.now();
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
            elapsedMs: performance.now() - clicked,
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
 * The window was in its clear first 0.3 s and said nothing yet, when Esc came in time for that; a
 * browser too slow to take the focus within 0.3 s is noted on the test instead, which then checks only
 * that the window closed.
 */
export function expectQuietWhenInTime(atEscape: AtEscape): void {
  if (atEscape.elapsedMs >= skeletonDelayMs) {
    test.info().annotations.push({
      type: "slow",
      description: `the window took the focus after ${Math.round(atEscape.elapsedMs)} ms`,
    });
    return;
  }
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
