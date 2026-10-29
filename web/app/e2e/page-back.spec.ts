import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { addressInProject, paths } from "../src/shared/paths.ts";

// M5 exit, package O: the link at the foot of a page that goes back up is on a computer and a tablet, and not on
// a phone, where the bar of the screen has Back already.

test.use({ sample: true });

/** A page with its address and the words of its link back. */
interface Place {
  readonly name: string;
  readonly open: (page: Page) => Promise<void>;
  readonly back: string;
}

/** The height of a card with nothing in it but padding: a card must be taller than this. */
const EMPTY_CARD = 48;

/** The height of the lowest card on the page. */
function smallestCard(page: Page): Promise<number> {
  return page.evaluate(() => {
    let least = Number.POSITIVE_INFINITY;
    for (const card of document.querySelectorAll("[data-part='card']")) {
      least = Math.min(least, card.getBoundingClientRect().height);
    }
    return least;
  });
}

const places: readonly Place[] = [
  {
    name: "a role",
    open: async (page) => {
      await page.goto(paths.roles);
      await page.locator("[data-part='link'][data-variant='row']").first().click();
    },
    back: t("roles.back"),
  },
  {
    name: "a field",
    open: async (page) => {
      await page.goto(paths.fields);
      await page.locator("[data-part='link'][data-variant='row']").first().click();
    },
    back: t("fields.back"),
  },
  {
    name: "a person",
    open: async (page) => {
      await page.goto(paths.people);
      await page.getByRole("option").first().click();
    },
    back: t("person.back"),
  },
  {
    name: "the settings of a project",
    open: async (page) => {
      await page.goto(addressInProject(paths.projectSettings, "APP"));
    },
    back: t("projects.settings-back"),
  },
];

test("ux_8 a page says Back at its foot on a computer and not on a phone", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  // One link on a computer and a tablet, none on a phone.
  const expected = { computer: 1, tablet: 1, phone: 0 }[layoutOf(page)];
  await inOrder(
    places.map((place) => async () => {
      await place.open(page);
      await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
      await expect(page.getByRole("link", { name: place.back }), place.name).toHaveCount(expected);
      // No card is left empty where the title and the link were hidden.
      expect(await smallestCard(page), `${place.name}: the smallest card`).toBeGreaterThan(
        EMPTY_CARD,
      );
    }),
  );
});
