import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import {
  paths,
  personAddress,
  projectAddress,
  roleAddress,
  addressInProject,
} from "../src/shared/paths.ts";
import { ada, addCodeApp, bob, invite, makeAdmin, makePerson } from "./support/accounts.ts";
import {
  expectGoodInEveryLook,
  expectGoodInEveryTheme as expectGoodInEveryThemeOnPage,
} from "./support/checks.ts";
import { setOrganizationSwitch, TWO_STEP_REQUIRED } from "./support/organization.ts";
import { expect, test } from "./support/fixtures.ts";
import { sendPassword, signIn } from "./support/flows.ts";

// The accessibility suite (TEST-A11Y): every address of `paths` is opened on a computer and on a
// phone (the two projects of the Playwright configuration), in the four themes, and again with the
// device asking for more contrast. `cargo xtask check only TEST-A11Y` fails when an address of
// `paths` is missing here. The first start (`paths.setup`) needs a server without an administrator
// and is in a11y-first-start.spec.ts.

/** The link of an invitation that nobody has used. */
let invitationLink = "";
/** The IDs of a person and of a role, for the pages of one. */
let somebody = "";
let someRole = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const session = await makePerson(server, admin, bob);
      somebody = idOf(await session.get("/api/v1/auth/me"));
      someRole = idOf(await admin.get("/api/v1/roles?limit=1"), "items");
      await addCodeApp(session, bob.password);
      invitationLink = await invite(admin, "guest@example.test");
      await admin.post("/api/v1/projects", { short_name: "WEB", name: "Website" });
      await admin.put("/api/v1/auth/sign-up-policy", { enabled: true, domains: ["example.test"] });
      await admin.dispose();
      await session.dispose();
    },
  },
});

/** The ID in an answer, or in the first item of its list `items`. */
function idOf(body: Readonly<Record<string, unknown>>, list?: string): string {
  const found: unknown = list === undefined ? body : Reflect.get(body, list);
  const first: unknown = Array.isArray(found) ? found[0] : found;
  const id: unknown = typeof first === "object" && first !== null ? Reflect.get(first, "id") : "";
  return typeof id === "string" ? id : "";
}

/** How a person gets to an address, and whether the theme is chosen on the page or put on it. */
interface Visit {
  readonly address: string;
  /** Opens the address, signed in as the administrator or not signed in. */
  readonly open: (page: Page) => Promise<void>;
  /** Pages before sign-in have the choice of theme at the bottom; the app keeps it in *My settings*. */
  readonly guest: boolean;
}

/** Waits until the page of the address shows its one heading. */
async function shown(page: Page): Promise<void> {
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
}

/** Signs in and goes to the address. */
function signedIn(address: string): Visit["open"] {
  return async (page) => {
    await signIn(page, ada);
    await page.goto(address);
    await shown(page);
  };
}

/** Signs in and goes to an address that is only known once the server has made what it names. */
function signedInAt(address: () => string): Visit["open"] {
  return async (page) => {
    await signIn(page, ada);
    await page.goto(address());
    await shown(page);
  };
}

/** Goes to the address without signing in. */
function asGuest(address: string): Visit["open"] {
  return async (page) => {
    await page.goto(address);
    await shown(page);
  };
}

/** The page is good in every theme: picked on the page before sign-in, put on it in the app. */
function expectGoodInEveryTheme(page: Page, guest: boolean): Promise<void> {
  return guest ? expectGoodInEveryThemeOnPage(page) : expectGoodInEveryLook(page);
}

const visits: readonly Visit[] = [
  { address: paths.home, open: signedIn(paths.home), guest: false },
  { address: paths.signIn, open: asGuest(paths.signIn), guest: true },
  {
    address: paths.secondStep,
    open: async (page) => {
      await sendPassword(page, bob);
      await expect(page).toHaveURL(new RegExp(paths.secondStep, "u"));
      await expect(
        page.getByRole("heading", { level: 1, name: t("second-step.title") }),
      ).toBeVisible();
    },
    guest: true,
  },
  { address: paths.signUp, open: asGuest(paths.signUp), guest: true },
  {
    address: paths.invitation,
    open: async (page) => {
      await page.goto(invitationLink);
      await shown(page);
    },
    guest: true,
  },
  { address: paths.mySettings, open: signedIn(paths.mySettings), guest: false },
  { address: paths.appearance, open: signedIn(paths.appearance), guest: false },
  { address: paths.region, open: signedIn(paths.region), guest: false },
  { address: paths.startPage, open: signedIn(paths.startPage), guest: false },
  { address: paths.shortcuts, open: signedIn(paths.shortcuts), guest: false },
  { address: paths.security, open: signedIn(paths.security), guest: false },
  { address: paths.sessions, open: signedIn(paths.sessions), guest: false },
  { address: paths.accessKeys, open: signedIn(paths.accessKeys), guest: false },
  {
    address: paths.organizationSettings,
    open: signedIn(paths.organizationSettings),
    guest: false,
  },
  { address: paths.people, open: signedIn(paths.people), guest: false },
  { address: paths.person, open: signedInAt(() => personAddress(somebody)), guest: false },
  { address: paths.groups, open: signedIn(paths.groups), guest: false },
  { address: paths.roles, open: signedIn(paths.roles), guest: false },
  { address: paths.role, open: signedInAt(() => roleAddress(someRole)), guest: false },
  { address: paths.signInRules, open: signedIn(paths.signInRules), guest: false },
  { address: paths.themes, open: signedIn(paths.themes), guest: false },
  { address: paths.status, open: signedIn(paths.status), guest: false },
  { address: paths.projects, open: signedIn(paths.projects), guest: false },
  { address: paths.fieldPermissions, open: signedIn(paths.fieldPermissions), guest: false },
  { address: paths.peopleVisibility, open: signedIn(paths.peopleVisibility), guest: false },
  { address: paths.checkAccess, open: signedIn(paths.checkAccess), guest: false },
  { address: paths.project, open: signedIn(projectAddress("WEB")), guest: false },
  {
    address: paths.projectSettings,
    open: signedIn(addressInProject(paths.projectSettings, "WEB")),
    guest: false,
  },
  {
    address: paths.projectRoles,
    open: signedIn(addressInProject(paths.projectRoles, "WEB")),
    guest: false,
  },
  {
    address: paths.projectFields,
    open: signedIn(addressInProject(paths.projectFields, "WEB")),
    guest: false,
  },
  {
    address: paths.projectPeople,
    open: signedIn(addressInProject(paths.projectPeople, "WEB")),
    guest: false,
  },
  {
    address: paths.projectCheckAccess,
    open: signedIn(addressInProject(paths.projectCheckAccess, "WEB")),
    guest: false,
  },
  { address: paths.frames, open: signedIn(paths.frames), guest: false },
];

for (const visit of visits) {
  test(`ux_04 ${visit.address} has no accessibility problems in the four themes, also with more contrast`, async ({
    page,
  }) => {
    await visit.open(page);
    await expectGoodInEveryTheme(page, visit.guest);
    // With the device asking for more contrast, text and lines are stronger and every theme is good.
    await page.emulateMedia({ contrast: "more" });
    await expectGoodInEveryTheme(page, visit.guest);
  });
}

// The screen of somebody who has to set up a second step exists only while the organization asks
// for one, so its visit switches that on and puts it back.
test(`ux_04 ${paths.setUpSecondStep} has no accessibility problems in the four themes, also with more contrast`, async ({
  page,
  server,
}) => {
  await setOrganizationSwitch(server, TWO_STEP_REQUIRED, true);
  try {
    await sendPassword(page, ada);
    await expect(page).toHaveURL(new RegExp(paths.setUpSecondStep, "u"));
    await shown(page);
    await expectGoodInEveryTheme(page, true);
    await page.emulateMedia({ contrast: "more" });
    await expectGoodInEveryTheme(page, true);
  } finally {
    await setOrganizationSwitch(server, TWO_STEP_REQUIRED, false);
  }
});
