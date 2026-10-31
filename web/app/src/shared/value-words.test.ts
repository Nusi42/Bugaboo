import { fieldTypeWords, secondsPerMinute, settingChoices } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import vectors from "../../../../tools/testdata/value-words.json";
import { formatTime, plainWords } from "./format.ts";
import { lengthTyped } from "./length.ts";
import { listText } from "./list-text.ts";
import { durationText, ownValueText } from "./own-value-text.ts";
import { deviceRegion, type Region } from "./region.ts";
import { renderSaid } from "./said.ts";
import { testWording } from "./wording-testing.ts";

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
    timeOfDay: found(
      settingChoices.regionTimeOfDay.find((choice) => choice === choices.timeOfDay),
      `the time of day ${choices.timeOfDay}`,
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

describe("ux_07_a_date_and_a_time_of_day_are_written_one_way_for_a_person_everywhere", () => {
  for (const vector of vectors.times) {
    it(`writes the time of day of ${vector.moment} for the ${vector.region} region`, () => {
      expect(plainSpaces(formatTime(vector.moment, regionOf(vector.region)))).toBe(vector.expected);
    });
  }
});

/** The number format of the vectors' decimal mark (`point`, `comma`), which are values of the setting too. */
function numberFormatOf(mark: string): Region["numberFormat"] {
  return found(
    settingChoices.regionNumberFormat.find((choice) => choice === mark),
    `the decimal mark ${mark}`,
  );
}

/** A side of a change of the vectors, which are words of a text. */
interface TextSide {
  readonly shown: string;
  readonly text: string;
}

/** A side of a change of the vectors as the module reads it; the vectors hold only texts. */
function textSideOf(
  side: TextSide | null,
): { readonly shown: "text"; readonly text: string } | null {
  if (side !== null && side.shown !== "text") {
    throw new Error(
      `The vectors name a side shown as ${side.shown}, which this test does not read.`,
    );
  }
  return side === null ? null : { shown: "text", text: side.text };
}

/** A change of the vectors: who, the field, and the two sides. */
interface VectorChange {
  readonly who: string;
  readonly field: string;
  readonly field_name: string;
  readonly before: TextSide | null;
  readonly after: TextSide | null;
}

/** A change of the vectors, as the module reads it. */
function changeOf(change: VectorChange) {
  return { ...change, before: textSideOf(change.before), after: textSideOf(change.after) };
}

/** What the screen makes of an answer of a reader: the minutes, the short form and the grid form it writes them in, or the sentence of the mistake. */
function shownOf(
  read: ReturnType<typeof testWording.readLength>,
): Record<string, string | number | null> {
  const { minutes, mistake } = read;
  return {
    minutes,
    short: minutes === null ? null : durationText(minutes * secondsPerMinute),
    grid: minutes === null ? null : testWording.lengthOnClock(minutes),
    mistake: mistake === null ? null : renderSaid(mistake),
  };
}

describe("time_04 the browser reads a typed length the way the server does", () => {
  for (const vector of vectors.lengths) {
    it(`reads ${JSON.stringify(vector.text)} with the ${vector.mark} mark`, () => {
      const typed = lengthTyped(vector.text, {
        ...deviceRegion,
        numberFormat: numberFormatOf(vector.mark),
      });
      expect(shownOf(testWording.readLength(typed))).toEqual(vector.read);
      expect(shownOf(testWording.readTimeEntry(typed))).toEqual(vector.entry);
    });
  }
  for (const vector of vectors.sentences) {
    it(`writes ${vector.minutes} minutes in a sentence`, () => {
      expect(renderSaid(testWording.lengthSentence(vector.minutes))).toBe(vector.expected);
    });
  }
  for (const vector of vectors.clocks) {
    it(`writes ${vector.minutes} minutes on the clock`, () => {
      expect(testWording.lengthOnClock(vector.minutes)).toBe(vector.expected);
    });
  }
});

describe("ntf_02 the digits of a text stay as they were written, on the screen and in the e-mail", () => {
  for (const vector of vectors.changes) {
    it(`writes ${vector.expected}`, () => {
      expect(renderSaid(testWording.changeSentence(changeOf(vector.change)))).toBe(vector.expected);
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
