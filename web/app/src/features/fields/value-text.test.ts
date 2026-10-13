import { describe, expect, it } from "vitest";

import { formatNumber } from "../../shared/format.ts";
import { durationText, ownValueText } from "../../shared/own-value-text.ts";
import {
  durationSeconds,
  isDecimalText,
  decimalToMillionths,
  isWholeNumberText,
  millionthsToDecimal,
  wordsOfLines,
} from "./value-text.ts";

describe("value-text", () => {
  it("cf_01_a_length_of_time_is_written_in_hours_and_minutes_and_read_back", () => {
    expect(durationText(5400)).toBe("1h 30m");
    expect(durationText(7200)).toBe("2h");
    expect(durationText(2700)).toBe("45m");
    expect(durationSeconds("1h 30m")).toBe(5400);
    expect(durationSeconds("90m")).toBe(5400);
    expect(durationSeconds("2h")).toBe(7200);
    expect(durationSeconds("45")).toBe(2700);
    expect(durationSeconds("soon")).toBeUndefined();
    expect(durationSeconds("")).toBeUndefined();
  });

  it("cf_01_numbers_are_checked_before_they_are_sent", () => {
    expect(isWholeNumberText("-12")).toBe(true);
    expect(isWholeNumberText("1.5")).toBe(false);
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
