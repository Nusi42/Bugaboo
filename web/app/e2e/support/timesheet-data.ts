import { dataOf } from "@bugaboo/api-client";

import { ownId, signInThroughApi } from "./accounts.ts";
import { ana, signedInAdmin } from "./access.ts";
import { createPerson, bob } from "./accounts.ts";
import { ben, createHr } from "./hr.ts";
import { createIssue, prepareWeb } from "./issues.ts";
import { inOrder } from "./sequence.ts";
import { type Instance } from "./server.ts";
import { addDays } from "../../src/shared/format.ts";
import { type ApiSession } from "./api.ts";
import { bookOn, chooseUtc, workedOn } from "./timesheet.ts";

/** How long to wait between two changes of work days, which the server takes one a second. */
const ONE_CHANGE_A_SECOND_MS = 1100;

/** A week in the past, where work days are recorded with a reason. */
export const PAST = "2026-10-05";

/** The Friday of that week: a day of its own for the footer, which the other tests leave alone. */
export const FRIDAY = addDays(PAST, 4);

/** The shop of the timesheet tests: the web fixtures, a person with *HR* and one without a permission. */
export async function prepareTimesheet(server: Instance): Promise<void> {
  await prepareWeb(server);
  const admin = await signedInAdmin(server);
  await createHr(server, admin);
  await (await createPerson(server, admin, ben)).dispose();
  await admin.dispose();
}

/** Ana records the Monday of the past week from 8:00 to 16:50 in UTC; her ID. */
export async function anaWorked(server: Instance): Promise<string> {
  const session = await signInThroughApi(server, ana);
  await chooseUtc(session);
  await workedOn(session, PAST, "08:00", "16:50");
  const id = await ownId(session);
  await session.dispose();
  return id;
}

/** The value of *Activity* and of *Cost center* an entry is booked with, by the ID of each field. */
async function starterValues(admin: ApiSession): Promise<Readonly<Record<string, string>>> {
  const starters = dataOf(await admin.client.GET("/api/v1/me/timer")).starters;
  const choose = async (mark: string, name: string): Promise<readonly [string, string]> => {
    const field = starters[mark] ?? "";
    const known = dataOf(
      await admin.client.GET("/api/v1/fields/{field}", { params: { path: { field } } }),
    );
    // The starter fields come with some choices (*Development*, *Testing*); others are added.
    const made = known.choices.some((each) => each.name === name)
      ? known
      : dataOf(
          await admin.client.POST("/api/v1/fields/{field}/choices", {
            params: { path: { field } },
            body: { version: known.version, name },
          }),
        );
    return [field, made.choices.find((each) => each.name === name)?.id ?? ""];
  };
  return Object.fromEntries([
    await choose("activity", "Development"),
    await choose("cost-center", "4100 Product"),
  ]);
}

/** Bob's week: five work days, one of them long, and time on two issues. */
export async function bobsWeek(server: Instance): Promise<void> {
  const admin = await signedInAdmin(server);
  const first = await createIssue(admin, "WEB", "Login after reset");
  const second = await createIssue(admin, "WEB", "Passkey sign-in");
  const values = await starterValues(admin);
  await admin.dispose();
  const session = await signInThroughApi(server, bob);
  await chooseUtc(session);
  const days = [0, 1, 2, 3, 4].map((day) => addDays(PAST, day));
  const ends = ["16:50", "17:10", "13:00", "18:30", "17:15"];
  // The server takes one change of a person's days a second.
  await inOrder(
    days.map((day, at) => async () => {
      await workedOn(session, day, "08:00", ends[at] ?? "17:00");
      await new Promise((resolve) => {
        setTimeout(resolve, ONE_CHANGE_A_SECOND_MS);
      });
    }),
  );
  await bookOn(session, first, days[0] ?? PAST, 120, values);
  await bookOn(session, first, days[1] ?? PAST, 90, values);
  await bookOn(session, second, days[3] ?? PAST, 80, values);
  await session.dispose();
}
