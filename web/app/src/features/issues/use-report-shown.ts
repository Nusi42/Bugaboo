import { useEffect } from "react";

import type { FoundLine } from "./found-issues.ts";
import { useListContext } from "./list-context.ts";

/** Tells the list which issues a group (or the whole list) shows now; it forgets them when the group goes. */
export function useReportShown(id: string, lines: readonly FoundLine[]): void {
  const { report } = useListContext();
  const written = lines.map((line) => line.id).join(",");
  useEffect(() => {
    report(id, written === "" ? [] : written.split(","));
    return () => {
      report(id, []);
    };
  }, [report, id, written]);
}
