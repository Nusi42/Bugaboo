import { fieldTypeWords, settingChoices } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import vectors from "../../../../tools/testdata/value-words.json";
import { changeValueText } from "./change-value.ts";
import { listText } from "./list-text.ts";
import { ownValueText } from "./own-value-text.ts";
import { deviceRegion, type Region } from "./region.ts";

const NARROW_SPACE_BEFORE_AM_PM = / (?<period>AM|PM)/u;

/** A space the browser may write before AM or PM is a plain one in the vectors. */
function plainSpaces(text: string | undefined): string | undefined {
  return text?.replace(NARROW_SPACE_BEFORE_AM_PM, " $<period>");
}

function found<T>(item: T | undefined, what: string): T {
  if (item === undefined) {
    throw new Error(`The vectors name ${what}, which the app does not know.`);
  }
  return item;
}

function regionOf(name: string): Region {
  const choices = found(new Map(Object.entries(vectors.regions)).get(name), `the region ${name}`);
  return {
    ...deviceRegion,
    timeZone: choices.timeZone,
    dateFormat: found(
      settingChoices.regionDateFormat.find((choice) => choice === choices.dateFormat),
      `the date format ${choices.dateFormat}`,
    ),
    numberFormat: found(
      settingChoices.regionNumberFormat.find((choice) => choice === choices.numberFormat),
      `the number format ${choices.numberFormat}`,
    ),
  };
}

/** The words the screen gives a value of one kind, for one region of the vectors. */
function wordsFor(kindWord: string, value: unknown, regionName: string): string | null {
  const kind = found(
    fieldTypeWords.find((word) => word === kindWord),
    `the kind ${kindWord}`,
  );
  return plainSpaces(ownValueText(kind, value, regionOf(regionName))) ?? null;
}

// The same file is read by `value_words.rs`: when one side changes alone, both tests fail.
describe("ntf_02 the screen words a value the way the e-mail does", () => {
  for (const vector of vectors.cases) {
    it(`gives ${vector.name} its words`, () => {
      expect(wordsFor(vector.kind, vector.value, vector.region)).toBe(vector.expected);
    });
  }
});

describe("ntf_02 a status, a priority and a list of names read the same in both", () => {
  for (const word of vectors.words) {
    it(`gives ${word.name} its word`, () => {
      expect(changeValueText(word.field, { shown: "text", text: word.text })).toBe(word.expected);
    });
  }

  for (const list of vectors.lists) {
    it(`lists ${list.names.length} names`, () => {
      expect(listText(list.names)).toBe(list.expected);
    });
  }
});
