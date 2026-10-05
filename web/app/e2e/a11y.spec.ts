import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths, projectAddress, projectSettingsAddress } from "../src/shared/paths.ts";
import { ada, addCodeApp, bob, invite, makeAdmin, makePerson } from "./support/accounts.ts";
import {
  expectGoodInEveryLook,
  expectGoodInEveryTheme as expectGoodInEveryThemeOnPage,
} from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { sendPassword, signIn } from "./support/flows.ts";

// The accessibility suite (TEST-A11Y): every address of `paths` is opened on a computer and on a
// phone (the two projects of the Playwright configuration), in the four themes, and again with the
// device asking for more contrast. `cargo xtask check only TEST-A11Y` fails when an address of
// `paths` is missing here. The first start (`paths.setup`) needs a server without an administrator
// and is in a11y-first-start.spec.ts.

/** The link of an invitation that nobody has used. */
let invitationLink = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      const session = await makePerson(server, admin, bob);
      await addCodeApp(session, bob.password);
      invitationLink = await invite(admin, "guest@example.test");
      await admin.post("/api/v1/projects", { short_name: "WEB", name: "Website" });
      await admin.put("/api/v1/auth/sign-up-policy", { enabled: true, domains: ["example.test"] });
      await admin.dispose();
      await session.dispose();
    },
  },
});

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
  { address: paths.shortcuts, open: signedIn(paths.shortcuts), guest: false },
  { address: paths.security, open: signedIn(paths.security), guest: false },
  { address: paths.sessions, open: signedIn(paths.sessions), guest: false },
  {
    address: paths.organizationSettings,
    open: signedIn(paths.organizationSettings),
    guest: false,
  },
  { address: paths.invitations, open: signedIn(paths.invitations), guest: false },
  { address: paths.status, open: signedIn(paths.status), guest: false },
  { address: paths.projects, open: signedIn(paths.projects), guest: false },
  { address: paths.project, open: signedIn(projectAddress("WEB")), guest: false },
  {
    address: paths.projectSettings,
    open: signedIn(projectSettingsAddress("WEB")),
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
