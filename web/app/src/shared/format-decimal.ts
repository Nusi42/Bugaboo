import { currentRegion, numberLocales, type Region } from "./region.ts";

/** A number as people write it with a fixed number of places: "0.40", "0,40". */
export function formatDecimal(
  value: number,
  places: number,
  region: Region = currentRegion(),
): string {
  return new Intl.NumberFormat(numberLocales[region.numberFormat], {
    minimumFractionDigits: places,
    maximumFractionDigits: places,
  }).format(value);
}
