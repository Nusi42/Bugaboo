import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";
import { expect } from "@playwright/test";

import { inOrder } from "./sequence.ts";

/** What a person has in a project, in the shape of the answer of the server. */
function standing(sees: boolean, heldBack: boolean, roles: readonly string[]) {
  return { sees, held_back: heldBack, roles };
}

const cleo = { shown: "name", id: "p1", name: "Cleo Coder" } as const;
const documents = { id: "x", short_name: "DOCS", name: "Documents" };
const interfaceProject = { id: "y", short_name: "API", name: "Interface" };
const noChange = { gains: [], loses: [], fields: [], people: [] };

/** A preview with every kind of sentence in it, and a person who could lift a No access. */
function previewWithEverything() {
  return {
    project: documents,
    to: null,
    preview: "made up for the test",
    blocked: true,
    truncated: false,
    summary: { will_change: 1, skipped: 0 },
    people: [
      {
        person: cleo,
        projects: [
          {
            project: documents,
            now: standing(false, true, []),
            after: standing(true, false, ["Viewer"]),
            ...noChange,
          },
          {
            project: interfaceProject,
            now: standing(true, false, ["Viewer"]),
            after: standing(false, false, []),
            ...noChange,
          },
        ],
      },
      {
        person: { shown: "someone" },
        projects: [
          {
            project: documents,
            now: standing(true, false, ["Viewer"]),
            after: standing(true, false, ["Team member"]),
            gains: ["create-sub-projects"],
            loses: [],
            fields: [
              {
                field: { code: "cost-center", message_key: "issue-field.cost-center" },
                before: { code: "hidden", message_key: "field-level.hidden" },
                after: { code: "can-see", message_key: "field-level.can-see" },
              },
            ],
            people: [],
          },
        ],
      },
    ],
    lifts: [{ person: cleo, project: documents }],
  };
}

/** Answers every preview of a move with the one above, so its sentences can be read. */
export async function answerPreviewsWithEverything(page: Page): Promise<void> {
  await page.route("**/api/v1/projects/*/move-preview", async (route) => {
    await route.fulfill({ json: previewWithEverything() });
  });
}

/** The sentences the preview above is told in, as people read them. */
export function everySentence(): readonly string[] {
  return [
    t("projects.line-gains", { roles: "Viewer" }),
    t("projects.line-loses"),
    t("projects.line-held-back-ends"),
    t("projects.line-roles", { before: "Viewer", after: "Team member" }),
    t("projects.line-can-now", { things: t("permission.create-sub-projects") }),
    t("projects.line-field", {
      field: t("issue-field.cost-center"),
      before: t("field-level.hidden"),
      after: t("field-level.can-see"),
    }),
    t("projects.line-lift", { person: "Cleo Coder", project: "Documents" }),
  ];
}

/** Every sentence is on the screen, and the move that is not allowed cannot be confirmed. */
export async function expectEverySentence(dialog: Locator): Promise<void> {
  await expect(dialog.getByText(t("projects.preview-blocked"))).toBeVisible();
  await inOrder(
    everySentence().map((sentence) => async () => {
      await expect(dialog.getByText(sentence)).toBeVisible();
    }),
  );
  // Under a person each project is named, then what changes in it.
  await expect(dialog.getByText(documents.name, { exact: true }).first()).toBeVisible();
  await expect(dialog.getByRole("button", { name: t("projects.move-confirm") })).toBeDisabled();
}
