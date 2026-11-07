import { describe, expect, it } from "vitest";

import { connectRequestOf } from "./request.ts";

describe("what an app puts in the address of the consent step", () => {
  it("ai_22 reads every member as the text it was written, also one that looks like a number or an object", () => {
    const request = connectRequestOf(
      "?client_id=0123&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A5000%2Fcb" +
        "&code_challenge=12345&code_challenge_method=S256&scope=see-only&state=%7B%22a%22%3A1%7D",
    );
    expect(request.client).toBe("0123");
    expect(request.code_challenge).toBe("12345");
    expect(request.state).toBe('{"a":1}');
    expect(request.redirect_uri).toBe("http://127.0.0.1:5000/cb");
  });

  it("ai_22 takes a member that is not there for an empty text and leaves the judging to the server", () => {
    const request = connectRequestOf("");
    expect(request.client).toBe("");
    expect(request.resource).toBe("");
  });
});
