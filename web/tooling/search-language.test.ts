import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  type Answer,
  moduleLocation,
  offsetsOf,
  startSearchLanguage,
  type Search,
  type VectorFile,
} from "@bugaboo/wasm-search";
import { describe, expect, it } from "vitest";

const vectorsLocation = new URL("../../tools/testdata/search-vectors.json", import.meta.url);

function isVectorFile(value: unknown): value is VectorFile {
  return (
    typeof value === "object" &&
    value !== null &&
    "catalogue" in value &&
    "cases" in value &&
    Array.isArray(value.cases) &&
    "places" in value &&
    Array.isArray(value.places)
  );
}

/** The vectors that `cargo xtask gen search-vectors` writes from the Rust code of the server. */
function readVectors(): VectorFile {
  const vectors: unknown = JSON.parse(readFileSync(fileURLToPath(vectorsLocation), "utf8"));
  if (!isVectorFile(vectors)) {
    throw new Error("The search vectors are not what the generator writes.");
  }
  return vectors;
}

/** The tree of an answer to a text that parses. */
function treeOf(answer: Answer): Search {
  if (answer.search === null) {
    throw new Error("The text does not parse.");
  }
  return answer.search;
}

const language = startSearchLanguage(() => readFileSync(moduleLocation));
const vectors = readVectors();

/** The cases that the checker accepts: Simple search edits their trees and shows the text they come to. */
const accepted = vectors.cases.filter((vector) => vector.answer.canonical !== null);

describe("qry_03 the browser answers every text as the server does", () => {
  it("has the cases of the server", () => {
    expect(vectors.cases.length).toBeGreaterThan(40);
    expect(vectors.places.length).toBeGreaterThan(20);
  });

  it.each(vectors.cases.map((vector) => [vector.text, vector] as const))(
    "the text %j comes to the same tree, canonical text and mistakes",
    (_text, vector) => {
      expect(language.check(vector.text, vectors.catalogue)).toEqual(vector.answer);
    },
  );

  it.each(accepted.map((vector) => [vector.text, vector] as const))(
    "the tree of %j prints back to its canonical text in the browser",
    (_text, vector) => {
      expect(language.print(treeOf(vector.answer))).toBe(vector.answer.canonical);
    },
  );

  it.each(vectors.places.map((vector) => [`${vector.text}|${vector.cursor}`, vector] as const))(
    "the suggestions at %j are those of the server",
    (_place, vector) => {
      expect(language.suggest(vector.text, vector.cursor, vectors.catalogue)).toEqual(
        vector.suggestions,
      );
    },
  );
});

describe("the module gives the browser what the server has", () => {
  it("parses without the fields and keeps what the checker would refuse", () => {
    const answer = language.parse("nothing is Open");
    expect(answer.mistakes).toEqual([]);
    expect(answer.canonical).toBe("nothing is Open");
    expect(language.check("nothing is Open", vectors.catalogue).mistakes).toHaveLength(1);
  });

  it("gives an own field its word by the rule of the language", () => {
    expect(language.ownFieldWord("Story points")).toBe("storyPoints");
  });

  it("prints a tree that Simple search edited", () => {
    const edited: Search = {
      ...treeOf(language.parse("status is Open and priority is high sort by due")),
      sort: [],
    };
    expect(language.print(edited)).toBe("status is Open and priority is high");
  });
});

describe("the places of the module are the places of the string", () => {
  it("counts units of the string where the module counts bytes", () => {
    const text = 'title contains "Größe" and 😀 is';
    const offsets = offsetsOf(text);
    const quote = text.indexOf('"');
    const bytes = new TextEncoder().encode(text.slice(0, quote)).length;
    expect(offsets.inString(bytes)).toBe(quote);
    expect(offsets.inBytes(quote)).toBe(bytes);
    // A place inside a pair of units is the place before the character.
    const emoji = text.indexOf("😀");
    expect(offsets.inBytes(emoji + 1)).toBe(offsets.inBytes(emoji));
  });

  it("marks the same word in a text with letters of other languages", () => {
    const text = 'title contains "Größe"';
    const answer = language.check(text, vectors.catalogue);
    const filter = answer.search?.filter;
    expect(filter?.type).toBe("field");
    expect(text.slice(filter?.span.start, filter?.span.end)).toBe(text);
    const found = language.suggest(
      'title contains "Gr',
      'title contains "Gr'.length,
      vectors.catalogue,
    );
    expect(found.replace).toEqual({ start: 15, end: 18 });
  });

  it("gives a mistake the place of its word in the string", () => {
    const text = "ünï is Open";
    const answer = language.check(text, vectors.catalogue);
    expect(answer.mistakes).toHaveLength(1);
    const place = answer.mistakes[0]?.span;
    expect(text.slice(place?.start, place?.end)).toBe("ünï");
  });
});
