import { t } from "@bugaboo/messages";

/**
 * What the button of the *Followers* row says: how many followers the pages read so far hold, in
 * the right plural ("1 follower", "3 followers"). When a next page exists the number is a lower
 * bound ("50 or more followers").
 */
export function followerCount(loaded: number, more: boolean): string {
  return more
    ? t("followers.count-more", { count: loaded })
    : t("followers.count", { count: loaded });
}
