import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  NameValueRow,
  partOf,
  part,
  ShowMoreOf,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { api } from "../../shared/api.ts";
import { followerCount } from "./follower-count.ts";
import { FollowerNames } from "./FollowerNames.tsx";
import { itemsOf, pageSize, usePaged } from "../../shared/paged.ts";
import { useFollowing, type Following } from "./use-following.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** The followers of an issue, as the person may see them, a page at a time. */
function useFollowers(issue: string) {
  return usePaged({
    queryKey: queryKeys.followers.list(issue),
    read: (after) =>
      api.client.GET("/api/v1/issues/{issue}/followers", {
        params: { path: { issue }, query: { limit: pageSize.short, ...after } },
      }),
  });
}

/** The line of the row: the count (or the words *You follow it*), and the button that turns it round. */
function FollowersLine({
  mine,
  count,
  onToggleNames,
}: {
  readonly mine: Following;
  /** The words of the count, or null where nobody follows. */
  readonly count: string | null;
  readonly onToggleNames: () => void;
}) {
  const reasonId = useId();
  return (
    <div {...partOf(part.followersLine)}>
      {/* Before the button, so that the button stays the last child of the line (it hangs over the end edge). */}
      {mine.why === undefined ? null : (
        <span id={reasonId} {...partOf(part.visuallyHidden)}>
          {mine.why}
        </span>
      )}
      <span {...partOf(part.followersState)}>
        {count === null ? null : (
          <Button look={buttonLook.quiet} onPress={onToggleNames}>
            {count}
          </Button>
        )}
        {mine.following && count === null ? (
          <span {...partOf(part.followersStateWords)}>{t("followers.following")}</span>
        ) : null}
      </span>
      <Button
        look={buttonLook.quiet}
        isPending={mine.pending}
        onPress={mine.turn}
        {...(mine.why === undefined ? {} : { "aria-describedby": reasonId })}
      >
        {mine.following ? t("followers.stop") : t("followers.follow")}
      </Button>
    </div>
  );
}

/**
 * The *Followers* row of the Details (ISS-07, *3 followers · Follow*): the number of followers, the button, and the
 * names when asked for. The words *You follow it* stand only where there is no count; with a count, the button *Stop following* says it, so the row stays one line in the narrow column. A row of the one list of the Details; each line starts with a button that hangs back
 * by its padding, so the text of the lines stands on one edge (R1). A reader who may not see followers gets neither names nor a number.
 * People who report, are assigned or comment follow by themselves.
 */
export function FollowersBlock({ readableId }: { readonly readableId: string }) {
  const list = useFollowers(readableId);
  const mine = useFollowing(readableId);
  const [shown, setShown] = useState(false);
  const people = itemsOf(list.data?.pages);
  const counted = people.length > 0;
  return (
    <NameValueRow name={t("followers.name")}>
      <FollowersLine
        mine={mine}
        count={counted ? followerCount(people.length, list.hasNextPage) : null}
        onToggleNames={() => {
          setShown(!shown);
        }}
      />
      {mine.problem === null ? null : <Alert tone={alertTone.danger}>{mine.problem}</Alert>}
      {shown ? <FollowerNames people={people} /> : null}
      {shown ? <ShowMoreOf label={t("followers.more")} list={list} /> : null}
    </NameValueRow>
  );
}
