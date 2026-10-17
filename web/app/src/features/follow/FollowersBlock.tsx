import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, NameValueRow, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { followerCount } from "./follower-count.ts";
import { FollowerNames } from "./FollowerNames.tsx";
import { itemsOf, pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys, refreshHistory } from "../../shared/query-keys.ts";

const reasonMessage = {
  reported: "followers.reason-reported",
  assigned: "followers.reason-assigned",
  commented: "followers.reason-commented",
  chose: "followers.reason-chose",
} as const;

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

/** *Follow* or *Following*: the person's own choice, with the reason when they follow. */
function FollowButton({ issue }: { readonly issue: string }) {
  const state = api.query.useQuery("get", "/api/v1/issues/{issue}/follow", {
    params: { path: { issue } },
  });
  const follow = api.query.useMutation("put", "/api/v1/issues/{issue}/follow", {
    onSuccess: refreshHistory,
  });
  const leave = api.query.useMutation("delete", "/api/v1/issues/{issue}/follow", {
    onSuccess: refreshHistory,
  });
  const params = { params: { path: { issue } } } as const;
  const following = state.data?.following === true;
  const reason = state.data?.reason;
  return (
    <>
      <Button
        look={buttonLook.quiet}
        isPending={follow.isPending || leave.isPending}
        onPress={() => {
          if (following) {
            leave.mutate(params);
          } else {
            follow.mutate(params);
          }
        }}
      >
        {following ? t("followers.stop") : t("followers.follow")}
      </Button>
      {following ? <span {...partOf(part.muted)}>{t("followers.following")}</span> : null}
      {reason === undefined || reason === null ? null : (
        <span {...partOf(part.muted)}>{t(reasonMessage[reason])}</span>
      )}
      {follow.isError ? <Alert tone={alertTone.danger}>{messageOf(follow.error)}</Alert> : null}
    </>
  );
}

/**
 * The *Followers* row of the Details (ISS-07, *3 followers · Follow*): the number of followers, the button, and the
 * names when asked for. A row of the one list of the Details; each line starts with a button that hangs back
 * by its padding, so the text of the lines stands on one edge (R1). A reader who may not see followers gets neither names nor a number.
 * People who report, are assigned or comment follow by themselves.
 */
export function FollowersBlock({ readableId }: { readonly readableId: string }) {
  const list = useFollowers(readableId);
  const [shown, setShown] = useState(false);
  const people = itemsOf(list.data?.pages);
  return (
    <NameValueRow name={t("followers.name")}>
      {people.length === 0 ? null : (
        <div {...partOf(part.row)}>
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              setShown(!shown);
            }}
          >
            {followerCount(people.length, list.hasNextPage)}
          </Button>
        </div>
      )}
      <div {...partOf(part.row)}>
        <FollowButton issue={readableId} />
      </div>
      {shown ? <FollowerNames people={people} /> : null}
      {shown && list.hasNextPage ? (
        <Button
          isPending={list.isFetchingNextPage}
          onPress={() => {
            void list.fetchNextPage();
          }}
        >
          {t("followers.more")}
        </Button>
      ) : null}
    </NameValueRow>
  );
}
