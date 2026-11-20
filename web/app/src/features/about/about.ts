import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { formatCalendarDay } from "../../shared/format.ts";

type Files = Schema<"AboutBody">["files"];
type File = Files[number];

/** How long this release gets security fixes: the newest release only (a preview), or until a day. */
export type Support =
  | { readonly kind: "newest-only" }
  | { readonly kind: "until"; readonly day: string };

/** The support period as the server states it (`about`): no last day means a preview. */
export function supportOf(period: Schema<"SupportPeriodBody">): Support {
  return period.fixes_until === null || period.fixes_until === undefined
    ? { kind: "newest-only" }
    : { kind: "until", day: period.fixes_until };
}

/** How long this release gets security fixes, in words (one place for the page and its tests). */
export function supportText(period: Schema<"SupportPeriodBody">): string {
  const support = supportOf(period);
  return support.kind === "until"
    ? t("about.support-until", { date: formatCalendarDay(support.day) })
    : t("about.support-preview");
}

/** The files a person may download, in the order the page lists them; any other file of the build is not offered. */
export const OFFERED = ["cyclonedx", "spdx", "notices"] as const;
export type DownloadKind = (typeof OFFERED)[number];

/** The words of the button of each download (one place for the page and its tests). */
export const downloadMessages = {
  cyclonedx: "about.download-cyclonedx",
  spdx: "about.download-spdx",
  notices: "about.download-notices",
} as const satisfies Readonly<Record<DownloadKind, string>>;

/** A file to download, with the kind that names its button. */
export interface Download {
  readonly kind: DownloadKind;
  readonly address: string;
}

/** The downloads of the build, in a fixed order, leaving out what this build does not carry. */
export function downloadKinds(files: Files): readonly Download[] {
  return OFFERED.flatMap((kind) => {
    const found: File | undefined = files.find((file) => file.kind === kind);
    return found === undefined ? [] : [{ kind, address: found.address }];
  });
}
