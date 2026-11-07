import { Alert, alertTone, Card, cardLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { useItemTrail } from "../shell/index.ts";
import { RunChanges } from "./RunChanges.tsx";
import { runActorLine, runStateText, runTitle, useRun, type Run } from "./runs.ts";
import { UndoRun } from "./UndoRun.tsx";

/** A run: what it is of, who made it and when, its state, its changes and *Undo the run*. */
export function RunPage({ run }: { readonly run: string }) {
  const read = useRun(run);
  if (read.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert>;
  }
  if (read.data === undefined) {
    return null;
  }
  return <RunContent run={run} found={read.data} />;
}

function RunContent({ run, found }: { readonly run: string; readonly found: Run }) {
  usePageTitle(runTitle(found));
  // Home, then the run: the trail on a computer, the title and Back on a phone.
  useItemTrail(paths.home, runTitle(found));
  return (
    <div {...partOf(part.stack)}>
      <Card
        heading={runTitle(found)}
        level={1}
        look={cardLook.pageHead}
        intro={runActorLine(found)}
      >
        <p {...partOf(part.muted)}>{runStateText(found)}</p>
      </Card>
      <RunChanges found={found} />
      <UndoRun run={run} found={found} />
    </div>
  );
}
