import { QueryObserver } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import { queryClient } from "./query-client.ts";
import { queryKeys, refreshHistory } from "./query-keys.ts";

describe("m3_f54 who changed a text is kept for one version of the issue", () => {
  it("has a key of its own for each issue, part and version, under the history", () => {
    const now = queryKeys.history.changedBy("ACME-1", "description", 4);
    expect(now).not.toEqual(queryKeys.history.changedBy("ACME-1", "description", 5));
    expect(now).not.toEqual(queryKeys.history.changedBy("ACME-1", "notes", 4));
    expect(now).not.toEqual(queryKeys.history.changedBy("ACME-2", "description", 4));
    // The key is not the one of a page of the history, so the two never share an answer.
    expect(now).not.toEqual(queryKeys.history.list("ACME-1", "changes"));
    // A change of the history reads it again with everything else under `history`.
    expect(now.slice(0, queryKeys.history.all.length)).toEqual(queryKeys.history.all);
  });
});

describe("iss_05 a read that began before a change is not the read the screen keeps", () => {
  it("reads the history again when a comment was written while its first read was on the way", async () => {
    // The first read of a page of the history is answered with what the server had before the
    // comment was committed. The change arrives while that read is still on the way.
    const key = queryKeys.history.list("ACME-1", "comments");
    const answers = ["nothing here yet", "the comment"] as const;
    const releases: (() => void)[] = [];
    const observer = new QueryObserver<string>(queryClient, {
      queryKey: key,
      queryFn: async () => {
        const answer = String(answers.at(releases.length));
        await new Promise<void>((done) => {
          releases.push(done);
        });
        return answer;
      },
    });
    const stop = observer.subscribe(() => {
      // The test reads the result by asking for it.
    });
    await expect.poll(() => releases.length).toBe(1);
    refreshHistory();
    // The old read finishes after the change; what it carries is not what the screen is left with.
    releases[0]?.();
    await expect.poll(() => releases.length).toBe(2);
    releases[1]?.();
    await expect.poll(() => observer.getCurrentResult().data).toBe("the comment");
    stop();
    queryClient.clear();
  });
});
