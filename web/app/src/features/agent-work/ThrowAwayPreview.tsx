import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { AppLongPreview } from "../../shared/app-long-preview.tsx";
import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { useRunChanges, useUndoPreview } from "../runs/index.ts";
import { runsOf, type AttemptRun, type Work } from "./agent-work.ts";
import { goesBackGroups, stayGroups } from "./throw-away-groups.tsx";

/** What the preview does not list yet: the skipped fields it names only by their number, and the next page of changes. */
function MoreOfRun({
  skippedMore,
  hasMore,
  isLoading,
  onMore,
}: {
  /** How many skipped fields the preview counts but does not list. */
  readonly skippedMore: number;
  readonly hasMore: boolean;
  readonly isLoading: boolean;
  readonly onMore: () => void;
}) {
  return (
    <>
      {skippedMore > 0 ? (
        <p {...partOf(part.muted)}>{t("runs.skipped-more", { count: skippedMore })}</p>
      ) : null}
      {hasMore ? (
        <div>
          <Button isPending={isLoading} onPress={onMore}>
            {t("runs.show-more")}
          </Button>
        </div>
      ) : null}
    </>
  );
}

/** What undoing the run of one attempt would do now: what goes back, item by item, and what stays and why. */
function RunPreview({ attempt }: { readonly attempt: AttemptRun }) {
  const read = useUndoPreview(attempt.run, true);
  const changes = useRunChanges(attempt.run);
  const preview = read.data;
  return (
    <div {...partOf(part.agentWorkSection)}>
      <h4 {...partOf(part.agentWorkLabel)}>
        {t("agent-work.attempt", { number: attempt.number })}
      </h4>
      {read.isError ? <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert> : null}
      {preview === undefined ? (
        read.isError ? null : (
          <p {...partOf(part.muted)}>{t("agent-work.throw-away-reading")}</p>
        )
      ) : (
        <WithWording>
          {(wording) => (
            <AppLongPreview
              summary={t("agent-work.throw-away-summary", {
                back: preview.goes_back,
                stay: preview.skipped_count,
              })}
              groups={[
                ...goesBackGroups(wording, itemsOf(changes.data?.pages), preview.skipped),
                ...stayGroups(preview.skipped),
              ]}
              label={t("agent-work.throw-away-list")}
            />
          )}
        </WithWording>
      )}
      <MoreOfRun
        skippedMore={preview === undefined ? 0 : preview.skipped_count - preview.skipped.length}
        hasMore={changes.hasNextPage}
        isLoading={changes.isFetchingNextPage}
        onMore={() => {
          void changes.fetchNextPage();
        }}
      />
    </div>
  );
}

/**
 * What *Throw away* would do (AI-35): for the run of every attempt, newest first, what goes back
 * item by item and what is skipped and why, in the one long preview of the app (as *Undo the run*
 * shows its changes), and the sentence that a branch or anything else outside Bugaboo stays as it is.
 */
export function ThrowAwayPreview({ work }: { readonly work: Work }) {
  return (
    <div {...partOf(part.agentWorkSection)}>
      {runsOf(work).map((attempt) => (
        <RunPreview key={attempt.run} attempt={attempt} />
      ))}
      <p {...partOf(part.muted)}>{t("agent-work.throw-away-outside")}</p>
    </div>
  );
}
