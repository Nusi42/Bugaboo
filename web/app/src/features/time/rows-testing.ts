// The choices of the starter fields for the tests of the rows of the timesheet.
import type { Schema } from "@bugaboo/api-client";

import { startersOf } from "./rows.ts";

type Entry = Schema<"TimeEntryItem">;
type Choice = Schema<"FieldChoiceBody">;

/** The choices of the two starter fields, as the fields the browser read have them. */
const choices: Readonly<Record<string, Choice>> = {
  "id-of-Development": {
    id: "id-of-Development",
    name: "Development",
    shown_as: "Development",
    search_words: ["Development"],
    color: "cat1",
    retired: false,
  },
  "id-of-Testing": {
    id: "id-of-Testing",
    name: "Testing",
    shown_as: "Testing",
    search_words: ["Testing"],
    color: "cat2",
    retired: false,
  },
  "id-of-cost-center": {
    id: "id-of-cost-center",
    name: "Product development",
    number: "4100",
    shown_as: "4100 Product development",
    search_words: ["4100", "Product development", "4100 Product development"],
    color: "cat3",
    retired: false,
  },
};

export const find = (_field: string, held: readonly string[]): readonly Choice[] =>
  Object.values(choices).filter((choice) => held.includes(choice.id));

export function entry(
  issue: string,
  readable: string,
  date: string,
  minutes: number,
  activity = "Development",
): Entry {
  return {
    id: `${issue}-${date}-${minutes}`,
    issue,
    readable_id: readable,
    date,
    minutes,
    note: "",
    own_fields: { a1: `id-of-${activity}`, c1: "id-of-cost-center" },
    own_field_names: {},
    version: 1,
  };
}

export const starters = startersOf({ activity: "a1", "cost-center": "c1" });
