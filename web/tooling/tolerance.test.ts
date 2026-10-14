import { describe, expect, it } from "vitest";

import { Tolerances } from "../app/e2e/support/tolerance.ts";

const holder = { name: "holder" };
const other = { name: "other" };
const line = "Failed to load resource: net::ERR_INTERNET_DISCONNECTED";
const lines = /^Failed to load resource: net::ERR_INTERNET_DISCONNECTED$/u;

describe("test_concurrent a cut network lets off its own console lines and nothing else", () => {
  it("expects the line of the cut context while it is offline", () => {
    const tolerances = new Tolerances();
    const network = tolerances.add(holder, lines);
    network.cut();
    expect(tolerances.expects(holder, line)).toBe(true);
  });

  it("does not expect it before the network is cut or after it is back", () => {
    const tolerances = new Tolerances();
    const network = tolerances.add(holder, lines);
    expect(tolerances.expects(holder, line)).toBe(false);
    network.cut();
    network.restored();
    expect(tolerances.expects(holder, line)).toBe(false);
  });

  it("does not expect the lines of another context", () => {
    const tolerances = new Tolerances();
    tolerances.add(holder, lines).cut();
    expect(tolerances.expects(other, line)).toBe(false);
  });

  it("does not expect a line the test did not name, also from the cut context", () => {
    const tolerances = new Tolerances();
    tolerances.add(holder, lines).cut();
    expect(
      tolerances.expects(holder, "Failed to load resource: WebKit encountered an internal error"),
    ).toBe(false);
    expect(tolerances.expects(holder, `${line} and a defect`)).toBe(false);
  });

  it("forgets everything for the next test", () => {
    const tolerances = new Tolerances();
    tolerances.add(holder, lines).cut();
    tolerances.clear();
    expect(tolerances.expects(holder, line)).toBe(false);
  });
});
