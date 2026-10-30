import { expect, type Locator, type Page } from "@playwright/test";

import { addressInProject, paths } from "../src/shared/paths.ts";
import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";

// M5 exit, the final pass (L3): every settings page, of the organization, of a project and My settings, opens with
// the same introduction: the small muted sentence under the heading, as wide as the column. Before, some pages had
// it muted and small and others in the size of reading text.

test.use({ sample: true });

/** The settings pages that have an introduction: one of each kind of page, and every page of each settings screen. */
const pages = [
  paths.people,
  paths.groups,
  paths.roles,
  paths.signInRules,
  paths.organizationSecurity,
  paths.themes,
  paths.fieldPermissions,
  paths.peopleVisibility,
  paths.checkAccess,
  paths.projects,
  paths.issueTypes,
  paths.fields,
  paths.relationKinds,
  paths.labels,
  paths.layouts,
  paths.workflows,
  paths.mailServer,
  paths.status,
  paths.appearance,
  paths.region,
  paths.startPage,
  paths.shortcuts,
  paths.security,
  paths.sessions,
  paths.accessKeys,
  addressInProject(paths.projectRoles, "APP"),
  addressInProject(paths.projectPeople, "APP"),
  addressInProject(paths.projectFields, "APP"),
  addressInProject(paths.projectCheckAccess, "APP"),
  addressInProject(paths.projectLabels, "APP"),
  addressInProject(paths.projectIssueTypes, "APP"),
] as const;

/** The look of an introduction that has to be the same on every page, and how much narrower than its card it is. */
interface IntroLook {
  readonly color: string;
  readonly size: string;
  readonly lineHeight: string;
  readonly narrower: number;
}

/** The look of an introduction on the page. */
function lookOf(intro: Locator): Promise<IntroLook> {
  return intro.evaluate((node) => {
    const style = getComputedStyle(node);
    const room = node.parentElement?.getBoundingClientRect().width ?? 0;
    return {
      color: style.color,
      size: style.fontSize,
      lineHeight: style.lineHeight,
      narrower: Math.round(room - node.getBoundingClientRect().width),
    };
  });
}

/** The colour of quiet words, as the theme draws them on this page. */
function mutedColor(page: Page) {
  return page.evaluate(() => {
    const probe = document.createElement("p");
    probe.dataset["part"] = "muted";
    document.body.append(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    return color;
  });
}

test("ux_5 every settings page opens with the one introduction, muted, small and as wide as its card", async ({
  page,
}) => {
  test.slow();
  await signIn(page, sampleAdministrator());
  const looks: IntroLook[] = [];
  await inOrder(
    pages.map((address) => async () => {
      await page.goto(address);
      const intro = partIn(page, "card.intro").first();
      await expect(intro, address).toBeVisible();
      looks.push(await lookOf(intro));
    }),
  );
  const muted = await mutedColor(page);
  expect(looks).toHaveLength(pages.length);
  expect(new Set(looks.map((look) => JSON.stringify(look)))).toEqual(
    new Set([JSON.stringify({ ...looks[0], narrower: 0 })]),
  );
  expect(looks[0]?.color).toBe(muted);
});
