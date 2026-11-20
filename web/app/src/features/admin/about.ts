import type { Schema } from "@bugaboo/api-client";

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

/** The files a person may download, in the order the page lists them; any other file of the build is not offered. */
const OFFERED = ["cyclonedx", "spdx", "notices"] as const;
export type DownloadKind = (typeof OFFERED)[number];

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
