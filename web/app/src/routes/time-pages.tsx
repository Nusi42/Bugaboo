import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { isCalendarDay } from "../shared/format.ts";
import { paths } from "../shared/paths.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";
import { OthersScreen } from "./others-screen.ts";
import { TimeOthers } from "./TimeOthers.tsx";

// The timesheet is loaded when it is first opened, so the first page stays small (coding-standards
// section 6). *Working hours of others* is in the same chunk.

const TimesheetScreen = lazyRouteComponent(
  () => import("../features/time/index.ts"),
  "TimesheetPage",
);

/** The week in the address, `?week=2026-10-05`: the first date of the week; none is the week of today. */
function weekOf(search: Readonly<Record<string, unknown>>): { week?: string | undefined } {
  const week = search["week"];
  return { week: typeof week === "string" && isCalendarDay(week) ? week : undefined };
}

/** *Timesheet*: the person's week; the address says which week is open. */
export const timeRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.time,
  validateSearch: weekOf,
  loader: loadsScreen(TimesheetScreen),
  component: function Timesheet() {
    const { week } = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <TimesheetScreen
        week={week}
        onWeek={(first: string) => {
          void navigate({ to: paths.time, search: { week: first } });
        }}
      />
    );
  },
});

/** *Working hours of others*: the picker of a person. */
export const timeOthersRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.timeOthers,
  validateSearch: weekOf,
  loader: loadsScreen(OthersScreen),
  component: function TimeOthersPicker() {
    return <TimeOthers path={paths.timeOthers} />;
  },
});

/** *Working hours of others*: the week of one person; the address says whose and which week. */
export const timeOfPersonRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.timeOfPerson,
  validateSearch: weekOf,
  loader: loadsScreen(OthersScreen),
  component: function TimeOfPerson() {
    return <TimeOthers path={paths.timeOfPerson} />;
  },
});
