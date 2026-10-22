import { fieldTypeWords, settingChoices } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import vectors from "../../../../tools/testdata/value-words.json";
import { plainWords } from "./format.ts";
import { listText } from "./list-text.ts";
import { ownValueText } from "./own-value-text.ts";
import { deviceRegion, type Region } from "./region.ts";

const NARROW_SPACE_BEFORE_AM_PM = / (?<period>AM|PM)/u;

/** A space the browser may write before AM or PM is a plain one in the vectors, and a date never breaks, so its hyphens and spaces do not either (`plainWords` takes that back). */
function plainSpaces(text: string | undefined): string | undefined {
  return text === undefined
    ? undefined
    : plainWords(text).replace(NARROW_SPACE_BEFORE_AM_PM, " $<period>");
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

// The vectors are written by `cargo xtask gen word-vectors` from the code the server words its
// e-mails with (`bb-core`, `wording`). The screen writes dates and numbers with `Intl`; when it
// differs from the server for a choice the server can know, these tests fail (ADR-0033).
describe("ntf_02 the screen words a value the way the server does", () => {
  for (const vector of vectors.cases) {
    it(`gives ${vector.name} its words`, () => {
      expect(wordsFor(vector.kind, vector.value, vector.region)).toBe(vector.expected);
    });
  }
});

describe("ntf_02 a list of names reads the same on the screen and in the e-mail", () => {
  for (const list of vectors.lists) {
    it(`lists ${list.names.length} names`, () => {
      expect(listText(list.names)).toBe(list.expected);
    });
  }
});
