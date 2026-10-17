import { locale } from "@bugaboo/messages";

/** The units a size is written in, from the smallest; each is 1024 times the one before. */
const units = ["byte", "kilobyte", "megabyte", "gigabyte"] as const;
const step = 1024;

/** The digits after the decimal point of a size: `1.5 MB`, and `25 MB` where it is whole. */
const decimals = 1;

/** A size in bytes as people write it: `312 byte`, `1.5 kB`, `25 MB`. */
export function formatSize(bytes: number): string {
  const power =
    bytes < step ? 0 : Math.min(Math.floor(Math.log(bytes) / Math.log(step)), units.length - 1);
  return new Intl.NumberFormat(locale, {
    style: "unit",
    unit: units[power] ?? units[0],
    unitDisplay: "short",
    maximumFractionDigits: decimals,
  }).format(bytes / step ** power);
}

/** The part of a name after its last dot in capitals (`REPORT.pdf` is `PDF`); nothing without one. */
export function extensionOf(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot <= 0 || dot === name.length - 1 ? "" : name.slice(dot + 1).toUpperCase();
}
