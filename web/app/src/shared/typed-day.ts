import { dayText } from "./format.ts";
import { dayProblem, readDay } from "./read-day.ts";
import { useRegion, type Region } from "./region.ts";
import { useSynced } from "./use-synced.ts";

/** A box for a day as it is typed: the text, the day it reads as, and what is wrong with it. */
export interface TypedDay {
  /** The text in the box, in the person's date format. */
  readonly typed: string;
  readonly setTyped: (text: string) => void;
  /** The day the text reads as (`2027-01-14`): empty for an empty box, none for text that is no day. */
  readonly day: string | undefined;
  /** How to type a day, while the text is none. */
  readonly problem: string | undefined;
  /** Reads another text the same way, as `day` reads the text in the box. */
  readonly read: (text: string) => string | undefined;
  readonly region: Region;
}

/** The day a text reads as: empty for an empty text, none for text that is no day. */
function dayOf(text: string, region: Region): string | undefined {
  return text.trim() === "" ? "" : readDay(text, region);
}

/**
 * The one way a box for a day is typed (TXT-DATES): it shows `value` (`2027-01-14`, or empty) in the
 * person's date format (*Region*), follows a new `value` while the person has not typed, and reads
 * what they type in that format (a year first is read in any format). Never the browser's own date
 * box, which writes the format of the device.
 */
export function useTypedDay(value: string): TypedDay {
  const region = useRegion();
  const [typed, setTyped] = useSynced(value === "" ? "" : dayText(value, region));
  return {
    typed,
    setTyped,
    day: dayOf(typed, region),
    problem: dayProblem(typed, region),
    read: (text) => dayOf(text, region),
    region,
  };
}
