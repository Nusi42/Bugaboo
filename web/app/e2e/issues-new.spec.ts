import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { descriptionBox } from "./support/description.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  expectQuietAtEscape,
  newIssueButton,
  newIssueDialog as dialog,
  openNewIssueDialog as openDialog,
  openNewIssueDialogFromTheMenu,
  pressNewIssue,
  pressNewIssueAndEscapeAtOnce,
  titleLabel,
  waitForNewIssueForm,
  prepareWeb,
  webIssues,
} from "./support/issues.ts";
import { onThisLayout } from "./support/layout.ts";
import {
  boxOf,
  expectWindowSeen,
  holdNewIssueCode,
  holdNewIssueLayout,
  pageSize,
  roomUnderWindowContent,
  windowIsClear,
  TALLER_THAN_THE_FORM_PX,
  windowCloseName,
  windowPlacesWhile,
} from "./support/windows.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The new-issue dialog (ISS-11, UX §4.5, M3 WP-06): project, type and fields, required fields that
// say so, and a draft that is kept when the window closes and is gone after Create.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("iss_11 the dialog asks for a title in words, and the issue is made with Create", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  await expect(dialog(page).getByLabel(titleLabel(), { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  // Without a title the second button waits, and says why to whoever reads it.
  await expect(
    dialog(page).getByRole("button", { name: t("issues.create-another"), exact: true }),
  ).toHaveAccessibleDescription(t("issues.missing-title"));
  await dialog(page).getByLabel(titleLabel(), { exact: true }).fill("A new problem");
  await dialog(page)
    .getByRole("button", { name: t("issues.create"), exact: true })
    .click();
  await expect(page.getByRole("heading", { name: /A new problem/u })).toBeVisible();
});

/** The most that may lie between the label of a field and its value. */
const MOST_BETWEEN_LABEL_AND_VALUE_PX = 16;

test("ux_4_5 R1 the name of Project stands above it, on the same edge as the name of Type", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  const project = await boxOf(dialog(page).getByText(t("issues.project"), { exact: true }));
  const type = await boxOf(dialog(page).getByText(t("issues.type"), { exact: true }));
  const name = await boxOf(dialog(page).getByText("Website", { exact: true }));
  expect(Math.abs(project.x - type.x)).toBeLessThanOrEqual(2);
  expect(name.y).toBeGreaterThan(project.y);
  expect(Math.abs(name.x - project.x)).toBeLessThanOrEqual(2);
  // The name of the project stands close under its label, as the value of any field does (M3-F189).
  expect(name.y - (project.y + project.height)).toBeLessThanOrEqual(
    MOST_BETWEEN_LABEL_AND_VALUE_PX,
  );
});

test("iss_11 typing the title does not move the buttons under the form", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  // The note about what is missing comes and goes with the title; a finger that is on its way to
  // Cancel must not find another button there.
  // Measured when the form has its fields.
  await expect(descriptionBox(dialog(page))).toBeVisible();
  const cancel = dialog(page).getByRole("button", { name: t("common.cancel"), exact: true });
  const before = await cancel.boundingBox();
  await dialog(page).getByLabel(titleLabel(), { exact: true }).fill("Half written");
  await expect(
    dialog(page).getByRole("button", { name: t("issues.create-another"), exact: true }),
  ).toBeEnabled();
  expect(await cancel.boundingBox()).toEqual(before);
});

test("ux_6_3 the new-issue window stands still on the first open of a project", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  // The first open of a project in a session reads its types, its layout and its draft while the window opens. A
  // window shown before they are there grows when its fields arrive and jumps to a new middle (a user's report); the
  // second open finds them read and stands still. Every frame in which the window is shown counts.
  const openUntilTheFieldsAreThere = async (): Promise<void> => {
    await openDialog(page);
    await expect(
      dialog(page).getByText(t("issue-field.priority"), { exact: true }).first(),
    ).toBeVisible();
  };
  const first = await windowPlacesWhile(page, openUntilTheFieldsAreThere);
  expect(first.length, "the window was seen").toBeGreaterThan(0);
  const placesSeen = new Set(first.map(({ x, y }) => `${x} ${y}`));
  expect([...placesSeen], "every frame shows the window in one place").toHaveLength(1);
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  const second = await windowPlacesWhile(page, openUntilTheFieldsAreThere);
  expect(second.at(-1), "the second open stands where the first ended").toEqual(first.at(-1));
});

test("ux_6_1 a window whose fields are slow says it is loading, shows grey shapes and stands still", async ({
  page,
}) => {
  // Taller than the filled form, so a window that went back to the middle would stand lower than the grey shapes did.
  await page.setViewportSize({ ...pageSize(page), height: TALLER_THAN_THE_FORM_PX });
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const release = await holdNewIssueLayout(page);
  const places = await windowPlacesWhile(page, async () => {
    await pressNewIssue(page);
    // After 0.3 s the window shows its bar and grey shapes, and says it is loading (UX §6.1).
    await expectWindowSeen(page);
    await expect(dialog(page).getByRole("status")).toHaveText(t("common.loading"));
    await expect(dialog(page).getByRole("button", { name: windowCloseName(page) })).toBeVisible();
    release();
    await expect(
      dialog(page).getByText(t("issue-field.priority"), { exact: true }).first(),
    ).toBeVisible();
  });
  const placesSeen = new Set(places.map(({ x, y }) => `${x} ${y}`));
  expect([...placesSeen], "the loading window and the filled one stand in one place").toHaveLength(
    1,
  );
  // A window at the top is as tall as its content once it is there, not as tall as the page (M7-F246).
  expect(await roomUnderWindowContent(page, dialog(page))).toBeLessThanOrEqual(1);
  // The form is there: the status says nothing more, and the focus is where the person types first.
  await expect(dialog(page).getByRole("status")).toHaveText("");
  await expect(dialog(page).getByLabel(titleLabel(), { exact: true })).toBeFocused();
});

test("ux_6_1 a window that is still loading closes with Escape and with its own button, and says so once", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const release = await holdNewIssueLayout(page);
  // After 0.3 s: grey shapes, the status says it is loading, and its own button closes it. This first
  // open also brings the code of the window, which is loaded when it is first opened.
  await pressNewIssue(page);
  await expectWindowSeen(page);
  await expect(dialog(page).getByRole("status")).toHaveText(t("common.loading"));
  await dialog(page)
    .getByRole("button", { name: windowCloseName(page) })
    .click();
  await expect(dialog(page)).toHaveCount(0);
  // The first 0.3 s: the window is there but clear, says nothing yet, and Esc closes it.
  expectQuietAtEscape(await pressNewIssueAndEscapeAtOnce(page));
  await expect(dialog(page)).toHaveCount(0);
  release();
});

test("ux_6_1 on the first open the window waits for its code as it waits for its fields, and does not jump", async ({
  page,
}) => {
  await page.setViewportSize({ ...pageSize(page), height: TALLER_THAN_THE_FORM_PX });
  const releaseCode = await holdNewIssueCode(page);
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  const releaseLayout = await holdNewIssueLayout(page);
  // The list behind is drawn first: a button that the page draws again while the window is open is a
  // new element, and the focus cannot go back to the one that is gone.
  await expect(page.getByText(webIssues.login, { exact: true }).first()).toBeVisible();
  // While its code is on the way the window is there at once: clear for 0.3 s, and Esc closes it.
  expectQuietAtEscape(await pressNewIssueAndEscapeAtOnce(page));
  await expect(dialog(page)).toHaveCount(0);
  await expect(newIssueButton(page), "the focus is back on New issue").toBeFocused();
  const places = await windowPlacesWhile(page, async () => {
    await newIssueButton(page).press("Enter");
    await expectWindowSeen(page);
    await expect(dialog(page).getByRole("status")).toHaveText(t("common.loading"));
    // The code arrives (its buttons show) while the fields are still on the way: still grey shapes.
    await releaseCode();
    await expect(
      dialog(page).getByRole("button", { name: t("issues.create"), exact: true }),
    ).toBeVisible();
    expect(await windowIsClear(page), "the real window goes on where the stand-in was").toBe(false);
    await expect(dialog(page).getByRole("status")).toHaveText(t("common.loading"));
    releaseLayout();
    await waitForNewIssueForm(page);
  });
  const placesSeen = new Set(places.map(({ x, y }) => `${x} ${y}`));
  expect(
    [...placesSeen],
    "the window stands in one place from its code to its fields",
  ).toHaveLength(1);
  expect(await roomUnderWindowContent(page, dialog(page))).toBeLessThanOrEqual(1);
  // The real window took over from the stand-in; closing it still puts the focus back on New issue.
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toHaveCount(0);
  await expect(newIssueButton(page), "the focus is back on New issue").toBeFocused();
});

test("cfg_01 a layout that cannot be read is said inside the window, which shows its form", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  // Refused here (a status the browser tests accept as an answer, not a defect); any failure takes the same way.
  await page.route("**/api/v1/layouts/form?*kind=new-issue*", (route) =>
    route.fulfill({ status: 403 }),
  );
  await pressNewIssue(page);
  await waitForNewIssueForm(page);
  await expect(dialog(page).getByText(t("layouts.unreadable"), { exact: true })).toBeVisible();
});

test("iss_11 a half-filled dialog keeps its draft, and the draft is gone after Create", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  // The window is shown once its fields are there; wait for them before pressing Cancel.
  await expect(descriptionBox(dialog(page))).toBeVisible();
  await dialog(page).getByLabel(titleLabel(), { exact: true }).fill("Half written");
  await expect(dialog(page).getByText(t("issues.draft-kept"))).toBeVisible();
  await dialog(page)
    .getByRole("button", { name: t("common.cancel"), exact: true })
    .click();
  await expect(dialog(page)).toHaveCount(0);
  // The same page, read again: the draft comes from the browser and from the server.
  await page.reload();
  await openDialog(page);
  await dialog(page)
    .getByRole("button", { name: /^Continue your draft from/u })
    .click();
  await expect(dialog(page).getByLabel(titleLabel(), { exact: true })).toHaveValue("Half written");
  await dialog(page)
    .getByRole("button", { name: t("issues.create"), exact: true })
    .click();
  await expect(page.getByRole("heading", { name: /Half written/u })).toBeVisible();
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  await expect(
    dialog(page).getByRole("button", { name: /^Continue your draft from/u }),
  ).toHaveCount(0);
});

test("iss_11 Create and start another keeps the project and the type and says what was made", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  await dialog(page).getByLabel(titleLabel(), { exact: true }).fill("First of two");
  await dialog(page)
    .getByRole("button", { name: t("issues.create-another"), exact: true })
    .click();
  await expect(dialog(page).getByText(/was created\./u)).toBeVisible();
  await expect(dialog(page).getByLabel(titleLabel(), { exact: true })).toHaveValue("");
});

test("ux_05 the key c opens the dialog, and a phone has the button in its bar", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("issues.page-title", { project: "Website" }) }),
  ).toBeVisible();
  await onThisLayout(page, {
    computer: async () => {
      await page.keyboard.press("c");
      await expect(dialog(page)).toBeVisible();
    },
    tablet: async () => {
      await page.keyboard.press("c");
      await expect(dialog(page)).toBeVisible();
    },
    phone: async () => {
      await openNewIssueDialogFromTheMenu(page);
    },
  });
});
