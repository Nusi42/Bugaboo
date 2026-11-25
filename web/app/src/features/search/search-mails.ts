import { dataOf, mailLastHour, type ReadonlyDeep, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { timeOfDayText } from "../../shared/format.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import type { Region } from "../../shared/region.ts";
import { dayRunsText, weekOf } from "../work-times/index.ts";

/** An e-mailed search as its person sees it (QRY-07). */
export type SearchMail = ReadonlyDeep<Schema<"SearchMailItem">>;

/** The person's e-mailed searches, with what the organization allows. */
export type SearchMailList = ReadonlyDeep<Schema<"SearchMailListBody">>;

/** When a saved search comes by e-mail: what the window *E-mail me this search* chooses. */
export interface SearchMailChoice {
  /** The weekdays, Monday first. */
  readonly days: readonly boolean[];
  /** The whole hour, 0 to 23, in the person's time zone. */
  readonly hour: number;
  readonly only_when_changed: boolean;
}

/** The hours of the day a person may choose, 0 to the last one the server takes. */
export const mailHours: readonly number[] = Array.from(
  { length: mailLastHour + 1 },
  (_, hour) => hour,
);

/** What a new e-mailed search starts with: Monday at 8:00, only when something is new or changed. */
export const firstChoice: SearchMailChoice = {
  days: [true, false, false, false, false, false, false],
  hour: 8,
  only_when_changed: true,
};

/** Something about e-mailed searches changed: the list is read again. */
function refreshSearchMails(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.search.mails() });
}

/** The person's e-mailed searches. */
export function useSearchMails() {
  return useQuery({
    queryKey: queryKeys.search.mails(),
    queryFn: async () => dataOf(await api.client.GET("/api/v1/search-mails")),
  });
}

/** The e-mailed search of one saved search, if the person has one. */
export function mailOf(list: SearchMailList | undefined, search: string): SearchMail | undefined {
  return list?.search_mails.find((mail) => mail.search === search);
}

/** *E-mail me this search*: gets a saved search by e-mail, or changes when it comes. */
export function useSetSearchMail() {
  return useMutation({
    mutationFn: async ({
      search,
      choice,
    }: {
      readonly search: string;
      readonly choice: SearchMailChoice;
    }) =>
      dataOf(
        await api.client.PUT("/api/v1/saved-searches/{search}/mail", {
          params: { path: { search } },
          body: {
            days: [...choice.days],
            hour: choice.hour,
            only_when_changed: choice.only_when_changed,
          },
        }),
      ),
    onSuccess: refreshSearchMails,
  });
}

/** *Stop*: the person no longer gets the saved search by e-mail. */
export function useStopSearchMail() {
  return useMutation({
    mutationFn: async (search: string) => {
      await api.client.DELETE("/api/v1/saved-searches/{search}/mail", {
        params: { path: { search } },
      });
    },
    onSuccess: refreshSearchMails,
  });
}

/** An hour of the day on the person's own clock (`8:00`, `8:00 AM`). */
export function hourText(hour: number, region: Region): string {
  return timeOfDayText(hour, 0, region);
}

/** When an e-mailed search comes: "Mon, Thu at 8:00", and that it waits for something new. */
export function whenText(mail: SearchMailChoice, region: Region): string {
  const values = {
    days: dayRunsText(weekOf(region.firstDay), mail.days),
    time: hourText(mail.hour, region),
  };
  return mail.only_when_changed
    ? t("search.mails-when-changed", values)
    : t("search.mails-when", values);
}
