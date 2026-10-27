import type { ReadableFields } from "@bugaboo/wasm-search";
import { testSearchLanguage } from "../../shared/search-language-testing.ts";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { barMode, useSearchBar, type SearchBarState } from "./use-search-bar.ts";

const language = testSearchLanguage;

const fields: ReadableFields = {
  fields: [
    {
      word: "status",
      kind: "choice",
      operators: ["is", "is_not", "in"],
      choices: ["Open", "Done"],
    },
  ],
};

/** The state of the bar the first time it is drawn for an address holding `text`. */
function stateFor(text: string): SearchBarState {
  const drawn: SearchBarState[] = [];
  function Probe() {
    drawn.push(useSearchBar({ language, fields, text, onText: () => {} }));
    return null;
  }
  renderToString(<Probe />);
  const [state] = drawn;
  if (state === undefined) {
    throw new Error("the bar was not drawn");
  }
  return state;
}

describe("qry_02 the state of the search bar follows the text of the address", () => {
  it("shows a search without mistakes as buttons, and can switch to text and back", () => {
    const bar = stateFor("status is Open");
    expect(bar.mode).toBe(barMode.simple);
    expect(bar.canBeSimple).toBe(true);
    expect(bar.search?.filter?.type).toBe("field");
    expect(bar.shown).toBe("status is Open");
  });

  it("shows a search with a mistake as text whatever was chosen, and cannot switch to buttons", () => {
    const bar = stateFor("status is Nope");
    expect(bar.mode).toBe(barMode.advanced);
    expect(bar.canBeSimple).toBe(false);
    expect(bar.search).toBeNull();
    expect(bar.answer.mistakes.length).toBeGreaterThan(0);
    expect(bar.switchTo(barMode.simple)).toBe(false);
  });

  it("takes the empty text for a search that finds everything", () => {
    const bar = stateFor("");
    expect(bar.canBeSimple).toBe(true);
    expect(bar.search?.filter).toBeNull();
  });
});
