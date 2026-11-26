import { describe, expect, it } from "vitest";

import {
  builtInCalendar,
  builtInView,
  presentationOf,
  viewFromPresentation,
  viewOf,
  withView,
} from "./list-view.ts";
import { readSearch, searchLocation } from "./search-address.ts";

const everywhere = builtInView({ everywhere: true });

describe("plan_09_shows_as_a_calendar_with_its_span_and_date_field_and_back", () => {
  it("reads the span and the date field of a calendar from the address and writes them back", () => {
    const state = readSearch({ search: { show: "calendar", span: "week", from: "start date" } });
    const view = viewOf(state, everywhere);
    expect(view.kind).toBe("calendar");
    expect(view.calendar).toEqual({ dates_field: "start date", span: "week" });
    expect(searchLocation(withView(readSearch({ search: {} }), view, everywhere)).search).toEqual({
      show: "calendar",
      span: "week",
      from: "start date",
    });
  });

  it("says nothing of *Due date* and a month, which a calendar has when it says nothing", () => {
    const state = readSearch({ search: { show: "calendar", from: "due date", span: "month" } });
    const view = viewOf(state, everywhere);
    expect(view.calendar).toEqual(builtInCalendar);
    expect(searchLocation(withView(readSearch({ search: {} }), view, everywhere)).search).toEqual({
      show: "calendar",
    });
  });

  it("reads a span it does not know as the defaults, and keeps the options in a saved search", () => {
    const state = readSearch({ search: { show: "calendar", span: "year" } });
    expect(viewOf(state, everywhere).calendar.span).toBe("month");
    const view = {
      ...everywhere,
      kind: "calendar",
      calendar: { dates_field: "begins", span: "week" },
    } as const;
    const saved = presentationOf(view);
    expect(saved.calendar).toEqual({ dates_field: "begins", span: "week" });
    expect(viewFromPresentation(saved, everywhere)).toEqual(view);
  });
});
