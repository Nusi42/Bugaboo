import { settingKeys, type SettingChoice } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";
import { setUpCompany } from "./readme-company.ts";
import { cargoBikeStory, createJob } from "./readme-issues.ts";
import { cargoBike, company, projects, savedSearches } from "./readme-organization-data.ts";
import { jobs } from "./readme-organization-jobs.ts";
import { setUpWork } from "./readme-work.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";

// Quillmoor Cycle Works, the made-up workshop the pictures of the front page show
// (`readme-organization-data.ts`), on an empty server, through the API.

export { accountOf } from "./readme-company.ts";

/** The issue the pictures open: the cargo bike, the repair after all the others. */
export const readmeIssue = `${projects.repairs.shortName}-${String(
  jobs.filter((job) => job.place === cargoBike.place).length + 1,
)}`;

/** The title of the issue the pictures open. */
export const readmeIssueTitle = cargoBike.title;

/** The project whose list the pictures show: the whole company. */
export const readmeProject = company.shortName;

/** The project of the repairs: its workflow, and its list on a phone, are in the pictures. */
export const readmeRepairs = projects.repairs.shortName;

/**
 * The theme of every picture and recording of the front page but the first, which shows all four:
 * Paper, the second light theme, so the pages look the same whatever the reader's GitHub shows.
 */
export const readmeTheme: SettingChoice<"appearanceLightTheme"> = "paper";

/** Marta sees the app always light, in the theme of the pictures (*My settings › Appearance*). */
async function showReadmeTheme(admin: ApiSession): Promise<void> {
  const choices = [
    [settingKeys.appearanceMode, "light" satisfies SettingChoice<"appearanceMode">],
    [settingKeys.appearanceLightTheme, readmeTheme],
  ] as const;
  await inOrder(
    choices.map(([key, value]) => async () => {
      const answer = await admin.client.PUT("/api/v1/me/settings/{key}", {
        params: { path: { key } },
        body: { value },
      });
      if (!answer.response.ok) {
        throw new Error(`The setting ${key} was not kept.`);
      }
    }),
  );
}

/**
 * Sets up Quillmoor Cycle Works on an empty server: the preparation of the pictures of the front
 * page. Marta Lindqvist, the administrator, signs in with `accountOf("marta")` and sees the app in
 * `readmeTheme`.
 */
export async function createReadmeOrganization(server: Instance): Promise<void> {
  const people = await setUpCompany(server);
  await showReadmeTheme(people.admin);
  const shop = { people, ways: await setUpWork(people.admin) };
  // The cargo bike comes last, so it is the newest and stands at the top of its group in the list.
  await inOrder(jobs.map((job) => () => createJob(shop, job)));
  await cargoBikeStory(shop);
  await inOrder(
    savedSearches.map((search) => async () => {
      await people.admin.client.POST("/api/v1/saved-searches", {
        body: { name: search.name, search_text: search.text },
      });
    }),
  );
  await inOrder(Object.values(people.sessions).map((session) => () => session.dispose()));
  await people.admin.dispose();
}
