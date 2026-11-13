import { ApiError } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { QueryGate } from "./QueryGate.tsx";

/** What the page behind the gate says: words of the test, not text for people. */
const PAGE = "behind the gate";

/** What a gate draws for a read with this answer and this refusal; the page itself says `page`. */
function drawn(data: unknown, error: unknown): string {
  return renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <QueryGate error={error} data={data} notFound={<p>{t("page.not-found.title")}</p>}>
        {() => <p>{PAGE}</p>}
      </QueryGate>
    </QueryClientProvider>,
  );
}

const notFound = new ApiError(404, {
  code: "resource.not-found",
  message_key: "error.resource.not-found",
  params: {},
  status: 404,
  title: "Not found",
  type: "about:blank",
});

describe("ux_01 the gate of a page shows a refusal, also beside an answer", () => {
  it("shows Not found for a read that was refused, and draws once", () => {
    expect(drawn(undefined, notFound)).toContain(t("page.not-found.title"));
  });

  it("shows the refusal of a read that has an answer too, not the page", () => {
    const markup = drawn({ id: "i1" }, notFound);
    expect(markup).toContain(t("page.not-found.title"));
    expect(markup).not.toContain(PAGE);
  });

  it("draws the page for a read with an answer and no refusal", () => {
    expect(drawn({ id: "i1" }, null)).toContain(PAGE);
  });
});
