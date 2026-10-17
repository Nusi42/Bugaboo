import { describe, expect, it } from "vitest";

import { followerCount } from "./follower-count.ts";

describe("iss_07 the number of followers uses the right plural", () => {
  it("says follower for one and followers for the rest", () => {
    expect(followerCount(1, false)).toBe("1 follower");
    expect(followerCount(2, false)).toBe("2 followers");
    expect(followerCount(12, false)).toBe("12 followers");
  });

  it("says or more when another page of followers exists", () => {
    expect(followerCount(50, true)).toBe("50 or more followers");
  });
});
