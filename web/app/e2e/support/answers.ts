import type { Route } from "@playwright/test";

// Answers the tests give in place of the server.

/**
 * The answer of a request the server refuses, as the server words it (problem details): the status, the error code
 * and the message key of the code, unless the test needs another one.
 */
export function refusal(
  status: number,
  code: string,
  messageKey = `error.${code}`,
): Parameters<Route["fulfill"]>[0] {
  return {
    status,
    contentType: "application/problem+json",
    body: JSON.stringify({
      type: `urn:bugaboo:error:${code}`,
      title: code,
      status,
      code,
      message_key: messageKey,
      params: {},
    }),
  };
}
