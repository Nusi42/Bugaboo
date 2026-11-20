import { dataOf, type Schema } from "@bugaboo/api-client";
import { useMutation, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import type { DataExport } from "../../shared/data-export.ts";
import { queryClient } from "../../shared/query-client.ts";

/** How often the page asks again while the file is being made: the job writes it within seconds to minutes. */
const WAITING_READ_MS = 2000;

/** Where the newest file of the person's own data stands. */
function myExportOptions() {
  return api.query.queryOptions("get", "/api/v1/me/data-export");
}

/** Whether the file is still being made. */
export function isWaiting(item: DataExport | null | undefined): boolean {
  return item?.status === "waiting";
}

/**
 * The newest file of the person's own data, read again every few seconds while it is being made, so the page shows the
 * link as soon as the job is done; then it stops asking.
 */
export function useMyExport() {
  return useQuery({
    ...myExportOptions(),
    refetchInterval: (
      query: Readonly<{ state: Readonly<{ data: Schema<"MyDataExportBody"> | undefined }> }>,
    ) => (isWaiting(query.state.data?.export) ? WAITING_READ_MS : false),
  });
}

/** *Download my data*: asks for the file; the answer is the export that is being made, shown at once. */
export function useAskForMyData() {
  return useMutation({
    mutationFn: async () => dataOf(await api.client.POST("/api/v1/me/data-export")),
    onSuccess: (made) => {
      queryClient.setQueryData(myExportOptions().queryKey, { export: made });
    },
  });
}
