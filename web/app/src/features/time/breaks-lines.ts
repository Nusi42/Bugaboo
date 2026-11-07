import type { Region } from "../../shared/region.ts";
import { breakSpan, clockText, zoneOfDay, type WorkDay } from "./sheet-model.ts";

/** The line a break is typed as: `12:10-12:40`, in the clock of the person and the zone of the day. */
function lineOf(
  span: { readonly start: string; readonly end: string | null },
  zone: string,
  region: Region,
): string {
  return breakSpan(clockText(span.start, zone, region), clockText(span.end, zone, region));
}

/** The breaks of a day, one line for each, as the window of the breaks starts with them. */
export function breaksAsLines(day: WorkDay | undefined, region: Region): string {
  const zone = zoneOfDay(day, region);
  return (day?.breaks ?? []).map((span) => lineOf(span, zone, region)).join("\n");
}
