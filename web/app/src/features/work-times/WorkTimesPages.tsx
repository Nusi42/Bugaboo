import { t } from "@bugaboo/messages";
import { Card, partOf, part } from "@bugaboo/ui";

import { paths, personAddress } from "../../shared/paths.ts";
import { personName } from "../../shared/person.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NotFoundPage, QueryGate, useItemTrail } from "../shell/index.ts";
import { Holidays } from "./Holidays.tsx";
import { useWorkTimes } from "./work-times.ts";
import { Sections } from "./WorkTimesSections.tsx";

/**
 * *My settings › Work times* (PLAN-11, PLAN-15, UX §4.13): when and how much the person usually
 * works, the public holidays that count, the days they are away and their shares in projects, so the
 * timeline can plan with them.
 */
export function OwnWorkTimesPage() {
  usePageTitle(t("nav.work-times"));
  const read = useWorkTimes();
  return (
    <QueryGate error={read.error} data={read.coming}>
      {(workTimes) => (
        <div {...partOf(part.stack)}>
          <Card heading={t("nav.work-times")} level={1} intro={t("work-times.intro")}>
            <Holidays workTimes={workTimes} />
          </Card>
          <Sections whose={undefined} read={read} workTimes={workTimes} otherName={undefined} />
        </div>
      )}
    </QueryGate>
  );
}

/**
 * The work times of somebody else (PLAN-11, UX §4.13, *For someone else*), opened from the person's
 * page by a holder of *Enter work times for others*: the same page, with the line that says the
 * person is told, and no note of an away time. The server decides what the reader sees and may do.
 */
export function PersonWorkTimesPage({ id }: { readonly id: string }) {
  const read = useWorkTimes(id);
  const name = read.coming === undefined ? undefined : personName(read.coming.person);
  const title = name === undefined ? t("nav.work-times") : t("work-times.title-other", { name });
  usePageTitle(title);
  useItemTrail(paths.people, name === undefined ? undefined : title);
  return (
    <QueryGate error={read.error} data={read.coming} notFound={<NotFoundPage />}>
      {(workTimes) => {
        const shown = personName(workTimes.person);
        return (
          <div {...partOf(part.stack)}>
            <Card
              heading={title}
              level={1}
              intro={t("work-times.intro-other", { name: shown })}
              back={{ href: personAddress(id), label: t("work-times.back-other", { name: shown }) }}
            >
              {workTimes.may_change ? (
                <p {...partOf(part.text)}>{t("work-times.changing-other", { name: shown })}</p>
              ) : null}
              <Holidays workTimes={workTimes} />
            </Card>
            <Sections whose={id} read={read} workTimes={workTimes} otherName={shown} />
          </div>
        );
      }}
    </QueryGate>
  );
}
