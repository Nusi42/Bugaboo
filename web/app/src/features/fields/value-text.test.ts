import { decimalWholeDigits } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { t } from "@bugaboo/messages";

import { wholeNumberIn } from "../../shared/number.ts";
import { testLengthReader } from "../../shared/wording-testing.ts";
import { field } from "./fields-testing.ts";
import { formatNumber } from "../../shared/format.ts";
import { durationText, ownValueText } from "../../shared/own-value-text.ts";
import {
  bound,
  boundDay,
  dayBound,
  sentFrom,
  isDecimalText,
  decimalToMillionths,
  millionthsToDecimal,
  wordsOfLines,
} from "./value-text.ts";

const reader = testLengthReader;
const duration = field({
  id: "hours",
  kind: "duration",
  look: "plain-text",
  name: "Hours logged",
  choices: [],
  rules: {},
});

describe("value-text", () => {
  it("cf_01_a_length_of_time_is_written_in_hours_and_minutes_and_read_back", () => {
    expect(durationText(5400)).toBe("1h 30m");
    expect(durationText(7200)).toBe("2h");
    expect(durationText(2700)).toBe("45m");
    // A length is read by the one reader of `bb-core`; a bare number is hours, like `1.5`.
    expect(sentFrom(duration, "1h 30m", reader)).toEqual({ value: 5400 });
    expect(sentFrom(duration, "90m", reader)).toEqual({ value: 5400 });
    expect(sentFrom(duration, "1.5", reader)).toEqual({ value: 5400 });
    expect(sentFrom(duration, "2", reader)).toEqual({ value: 7200 });
    // A length of an own field may be longer than one time entry.
    expect(sentFrom(duration, "40h", reader)).toEqual({ value: 144_000 });
    expect(sentFrom(duration, "", reader)).toEqual({ value: null });
    expect(sentFrom(duration, "soon", reader)).toEqual({
      problem: "Type a length of time like 1:30, 1.5 or 1h 30m.",
    });
    // Without the module there is no reader, and the box says so.
    expect(sentFrom(duration, "2h")).toEqual({ problem: t("common.words-failed") });
  });

  it("cf_01_numbers_are_checked_before_they_are_sent", () => {
    expect(wholeNumberIn("-12")).toBe(-12);
    expect(wholeNumberIn("1.5")).toBeNull();
    expect(isDecimalText("1.5")).toBe(true);
    expect(isDecimalText("1.1234567")).toBe(false);
    expect(isDecimalText("12")).toBe(true);
  });

  it("cf_01_a_decimal_is_kept_in_millionths_and_comes_back_the_same", () => {
    expect(decimalToMillionths("12.5")).toBe(12_500_000);
    expect(decimalToMillionths("-0.25")).toBe(-250_000);
    expect(decimalToMillionths("7")).toBe(7_000_000);
    expect(decimalToMillionths("x")).toBeUndefined();
    expect(millionthsToDecimal(12_500_000)).toBe("12.5");
    expect(millionthsToDecimal(-250_000)).toBe("-0.25");
    expect(millionthsToDecimal(7_000_000)).toBe("7");
  });

  it("cf_04_choices_are_typed_one_to_a_line", () => {
    expect(wordsOfLines("Minor\n\n  Major \nCritical")).toEqual(["Minor", "Major", "Critical"]);
  });
});

describe("ownValueText", () => {
  it("iss_06_a_value_the_screen_words_itself_is_worded_as_the_issue_page_does", () => {
    expect(ownValueText("yes-no", true)).toBe("Yes");
    expect(ownValueText("yes-no", false)).toBe("No");
    expect(ownValueText("duration", 5400)).toBe("1h 30m");
    expect(ownValueText("whole-number", 1234)).toBe(formatNumber(1234));
    expect(ownValueText("decimal", "12.5")).toBe("12.5");
    expect(ownValueText("date", "2026-10-10")).toMatch(/2026/u);
    expect(ownValueText("date-time", "2026-10-10T08:00:00Z")).toMatch(/2026/u);
    expect(ownValueText("yes-no", "maybe")).toBeUndefined();
    expect(ownValueText("duration", null)).toBeUndefined();
  });
});

describe("value-text digits", () => {
  it("cf_01_a_number_with_too_many_digits_is_none", () => {
    expect(sentFrom(duration, `1${"0".repeat(400)}`, reader)).toHaveProperty("problem");
    expect(decimalToMillionths("123456789.5")).toBe(123_456_789_500_000);
    // The server keeps a decimal in an i64 of millionths: 13 whole digits are its limit, and a
    // stored 1,234,567,890.5 can be saved again.
    expect(decimalWholeDigits).toBe(13);
    expect(isDecimalText("1234567890.5")).toBe(true);
    expect(decimalToMillionths("1234567890.5")).toBe(1_234_567_890_500_000);
    expect(isDecimalText("9223372036854.775807")).toBe(true);
    expect(isDecimalText("92233720368547")).toBe(false);
    // Beyond the exact whole numbers of a JavaScript number the millionths are none, not rounded.
    expect(decimalToMillionths("9223372036854.775807")).toBeUndefined();
    expect(isDecimalText(`1${"0".repeat(400)}`)).toBe(false);
  });
});

describe("cf_03 the bounds of a range of days", () => {
  const microsPerDay = 86_400_000_000;

  it("keeps a Date in days, the same day for both ends", () => {
    expect(dayBound("date", "2026-12-31", bound.lowest)).toBe(20_818);
    expect(dayBound("date", "2026-12-31", bound.highest)).toBe(20_818);
  });

  it("lets the highest day of a Date and time stand for the whole day", () => {
    const start = 20_818 * microsPerDay;
    expect(dayBound("date-time", "2026-12-31", bound.lowest)).toBe(start);
    expect(dayBound("date-time", "2026-12-31", bound.highest)).toBe(start + microsPerDay - 1);
    expect(boundDay("date-time", start + microsPerDay - 1)).toBe("2026-12-31");
  });

  it("has no bound for text that is no day of the calendar", () => {
    expect(dayBound("date", "2026-02-30", bound.lowest)).toBeUndefined();
  });
});
