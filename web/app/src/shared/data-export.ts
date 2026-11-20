import { dataOf, type Schema } from "@bugaboo/api-client";

import { api } from "./api.ts";

/** A file of *Download my data*, as the server sends it. */
export type DataExport = Schema<"DataExportItem">;

/**
 * Downloads the file of a ready export: the server gives the person who asked for it an address that is good for five
 * minutes, and the browser fetches it as a file to save (ADR-0044, decision 2). *My data* and *Data exports* download
 * this way.
 */
export async function downloadExport(id: string): Promise<void> {
  const link = dataOf(
    await api.client.GET("/api/v1/data-exports/{export}/address", {
      params: { path: { export: id } },
    }),
  );
  window.location.assign(link.download_url);
}
