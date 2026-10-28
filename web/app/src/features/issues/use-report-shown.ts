import { useEffect } from "react";

import type { FoundLine } from "./found-issues.ts";
import { useListContext } from "./list-context.ts";

/** Tells the list which issues a group (or the whole list) shows now; it forgets them when the group goes. */
export function useReportShown(id: string, lines: readonly FoundLine[]): void {
  const { report } = useListContext();
  // An ID and a readable ID have no space and no comma, so this tells the same lines apart.
  const written = lines.map((line) => `${line.id} ${line.key}`).join(",");
  useEffect(() => {
    report(
      id,
      written === ""
        ? []
        : written.split(",").map((each) => {
            const [issue = "", key = ""] = each.split(" ");
            return { id: issue, key };
          }),
    );
    return () => {
      report(id, []);
    };
  }, [report, id, written]);
}
