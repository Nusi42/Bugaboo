import settings from "./settings.json" with { type: "json" };
import { sampleStartMs } from "../support/sample.ts";

// What the pictures of the views stand on (architecture §11, *Visual tests*): the settings of the
// suite, the fixed search and the moment of the clock of the browser.

/** The settings of the suite, which the Playwright configuration and TEST-VISUAL read too. */
export const visual = settings;

/** Why the pictures are skipped on another system than Windows: the fonts differ. */
export const onlyOnWindows =
  "The pictures are taken and compared on Windows only: the fonts of other systems differ.";

/** The one search every view shows: the issues of the plan of the sample's goal. */
export const fixedSearch = "project is APP";

const MS_PER_MINUTE = 60_000;
const MINUTES_PER_HOUR = 60;

/** The minutes the time zone of the settings is ahead of UTC at `moment`. */
function offsetMinutes(moment: number): number {
  const name = new Intl.DateTimeFormat("en-US", {
    timeZone: settings.timeZone,
    timeZoneName: "longOffset",
  }).format(moment);
  const found = /GMT(?<sign>[+-])(?<hours>\d\d):(?<minutes>\d\d)/u.exec(name)?.groups;
  if (found === undefined) {
    return 0;
  }
  const minutes = Number(found["hours"]) * MINUTES_PER_HOUR + Number(found["minutes"]);
  return found["sign"] === "-" ? -minutes : minutes;
}

/**
 * The moment the clock of the browser stands at: the hour of the settings, in their time zone, on
 * the day the fixed sample ends.
 */
export function picturedAt(): number {
  const end = sampleStartMs("fixed");
  const day = new Intl.DateTimeFormat("en-CA", { timeZone: settings.timeZone }).format(end);
  const [year, month, date] = day.split("-").map(Number);
  const asUtc = Date.UTC(year ?? 0, (month ?? 1) - 1, date ?? 1, settings.hour);
  return asUtc - offsetMinutes(asUtc) * MS_PER_MINUTE;
}
