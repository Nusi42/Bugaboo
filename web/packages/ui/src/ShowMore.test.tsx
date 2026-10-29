import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { doNothing } from "./do-nothing.ts";
import { SearchPicker } from "./SearchPicker.tsx";
import { ShowMore } from "./ShowMore.tsx";

/** Words of the tests, not text for people. */
const words = {
  clear: "Clear",
  more: "Show more people",
  legend: "People",
  search: "Search",
  found: "Found",
  none: "",
};

describe("ux_5 the one Show more", () => {
  it("is a plain button on a row of its own", () => {
    const markup = renderToStaticMarkup(
      <ShowMore label={words.more} pending={false} onPress={doNothing} />,
    );
    expect(markup).toContain('data-part="row"');
    expect(markup).toContain('data-part="button"');
    expect(markup).not.toContain("quiet");
    expect(markup).toContain(words.more);
  });

  it("is the one a picker uses under its list", () => {
    const markup = renderToStaticMarkup(
      <SearchPicker
        legend={words.legend}
        searchLabel={words.search}
        clearLabel={words.clear}
        listLabel={words.found}
        query={words.none}
        onQuery={doNothing}
        failed={undefined}
        nothing={undefined}
        more={{ label: words.more, pending: false, press: doNothing }}
      >
        {null}
      </SearchPicker>,
    );
    expect(markup).toContain(words.more);
    expect(markup).not.toContain("quiet");
  });
});
