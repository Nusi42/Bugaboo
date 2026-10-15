import { firstVersion } from "@bugaboo/api-client";

/** The version of a record as the server counts it: the first one while the answer has none yet. */
export function versionOf(record: { readonly version?: number | null | undefined }): number {
  return record.version ?? firstVersion;
}
