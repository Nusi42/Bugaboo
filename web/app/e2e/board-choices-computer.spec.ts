import { savedSearchAddress } from "../src/shared/paths.ts";
import { ada, signInThroughApi, type Instance } from "./support/accounts.ts";
import {
  cardOf,
  cards,
  columnOf,
  gripOf,
  openMoveTo,
  prepareBoard,
  saveBoard,
} from "./support/board.ts";
import { createOwnField, setOwnValue } from "./support/fields.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { letGoOn, takeUp } from "./support/hand-drag.ts";
import { ownValueOf } from "./support/timeline-api.ts";

// A board whose columns are the choices of an own field (PLAN-06, UX §4.6): a card moved to another column, by
// *Move to …* and by a drag, takes that choice. The route names a column by the name of its choice, so the test
// holds the screen and the route to the same choice (M10-F93).

const FIELD = "Severity";
const choices = { minor: "Minor", major: "Major", critical: "Critical" } as const;

/** The board by the field, its field, and the IDs of its choices, after the preparation. */
let board = "";
let field = "";
let choiceIds: Readonly<Record<string, string>> = {};

async function prepare(server: Instance): Promise<void> {
  await prepareBoard(server);
  const admin = await signInThroughApi(server, ada);
  const made = await createOwnField(admin, FIELD, "pick-one", Object.values(choices));
  field = made.id;
  choiceIds = Object.fromEntries(made.choices.map((choice) => [choice.name, choice.id]));
  await setOwnValue(admin, cards.open, field, choiceIds[choices.minor]);
  await setOwnValue(admin, cards.other, field, choiceIds[choices.minor]);
  board = await saveBoard(admin, `Board by ${FIELD}`, {
    columns: FIELD.toLowerCase(),
    rows: null,
    card_limits: [],
  });
  await admin.dispose();
}

test.use({ preparation: { run: prepare } });

test("plan_06_a_card_moved_to_a_column_of_a_choice_field_takes_that_choice", async ({
  page,
  server,
}) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(board));
  await expect(columnOf(page, choices.minor).getByText(cards.open)).toBeVisible();
  // With Move to …
  const window = await openMoveTo(page, cards.open);
  await window.getByRole("button", { name: choices.major, exact: true }).click();
  await expect(columnOf(page, choices.major).getByText(cards.open)).toBeVisible();
  expect(await ownValueOf(server, cards.open, field)).toBe(choiceIds[choices.major]);
  // With a drag.
  await expect(cardOf(page, cards.other)).toBeVisible();
  await takeUp(page, gripOf(page, cards.other));
  await letGoOn(page, columnOf(page, choices.critical));
  await expect(columnOf(page, choices.critical).getByText(cards.other)).toBeVisible();
  expect(await ownValueOf(server, cards.other, field)).toBe(choiceIds[choices.critical]);
});
