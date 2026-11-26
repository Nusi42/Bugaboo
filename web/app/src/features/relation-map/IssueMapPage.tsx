import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, DelayedSkeleton, part, partOf } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { issueAddress } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { relationKindLinesOf, useRelationKinds } from "../relation-kinds/index.ts";
import { useIssueMap, type KindsShown, type StepsAway } from "./map-data.ts";
import { cutWords } from "./map-view.ts";
import { RelationMap } from "./RelationMap.tsx";

/** What the address of the map of an issue says: how far it walks and which kinds it shows. */
export interface IssueMapAsk {
  readonly steps: StepsAway;
  readonly shown: KindsShown;
}

export interface IssueMapPageProps {
  /** The readable ID of the start, as the address names it. */
  readonly address: string;
  readonly ask: IssueMapAsk;
  /** Writes a new address: other steps or kinds, or another start (*Start from here*). */
  readonly onAsk: (issue: string, ask: IssueMapAsk) => void;
}

/**
 * The relation map of an issue (ISS-16, UX §4.15), at `/issues/WEB-142/map`: everything connected
 * to it, step by step, as far as the person may see. *Start from here* makes another issue the
 * start and keeps the steps and the kinds; the address holds all of it, so back walks back.
 */
export function IssueMapPage({ address, ask, onAsk }: IssueMapPageProps) {
  const title = t("relation-map.title", { key: address });
  usePageTitle(title);
  const kindsRead = useRelationKinds();
  const kinds = relationKindLinesOf(kindsRead.data?.pages).map((kind) => kind.id);
  const map = useIssueMap({
    issue: address,
    steps: ask.steps,
    shown: ask.shown,
    kinds: kindsRead.data === undefined ? undefined : kinds,
  });
  const back = { href: issueAddress(address), label: t("relation-map.back", { key: address }) };
  return (
    // A wide page from its first frame: the picture takes the room the window leaves (UX §6.4).
    <div {...partOf(part.mapPage)}>
      <Card heading={title} level={1} back={back}>
        {map.isError ? <Alert tone={alertTone.danger}>{messageOf(map.error)}</Alert> : null}
        {map.data === undefined ? (
          <DelayedSkeleton lines={4} />
        ) : (
          <RelationMap
            answer={map.data}
            kindPages={kindsRead.data?.pages}
            label={(count) => t("relation-map.picture-label", { key: address, count })}
            cut={cutWords.issue()}
            nothing={t("relation-map.nothing", { key: address })}
            steps={{
              value: ask.steps,
              onChange: (steps) => {
                onAsk(address, { ...ask, steps });
              },
            }}
            shown={ask.shown}
            onShown={(shown) => {
              onAsk(address, { ...ask, shown });
            }}
            onStartFrom={(key) => {
              onAsk(key, ask);
            }}
          />
        )}
      </Card>
    </div>
  );
}
