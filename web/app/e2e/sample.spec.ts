import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { addressWith, expectHome, sendPassword, sendSecondStepCode } from "./support/flows.ts";
import { samplePeople, sampleTeamMember, sampleWithCodeApp } from "./support/sample.ts";
import { codeApp } from "./support/totp.ts";
import { paths } from "../src/shared/paths.ts";

/** The people of the sample (architecture §11), and how many of them left in part 8. */
const SAMPLE_PEOPLE = 50;
const PEOPLE_WHO_LEFT = 2;

test.use({ sample: true });
test.describe.configure({ mode: "serial" });

test("sample a Team member of the sample organization signs in with the password from the file", async ({
  page,
}) => {
  // The 50 people of the sample but the two who left (part 8), who cannot sign in.
  expect(samplePeople()).toHaveLength(SAMPLE_PEOPLE - PEOPLE_WHO_LEFT);
  const member = sampleTeamMember();
  expect(member.email.endsWith("@bugaboo.example")).toBe(true);
  await sendPassword(page, member);
  await expectHome(page, member);
});

test("sample a wrong password does not open the account of a sample person", async ({ page }) => {
  await sendPassword(page, sampleTeamMember(), "not-the-password");
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.wrong-credentials"));
});

test("sample a person with a code app is asked for a code and gets in with it", async ({
  page,
}) => {
  const { person, secret } = sampleWithCodeApp();
  await sendPassword(page, person);
  await expect(page).toHaveURL(addressWith(paths.secondStep));
  await sendSecondStepCode(page, await codeApp(secret).next());
  await expectHome(page, person);
});
