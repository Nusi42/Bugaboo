// Outside the browser app (unit tests, end-to-end tests) every text is wanted at once, so this
// entry loads all parts first. The browser build never takes it: `package.json` offers it to Node
// only.
import "./generated/all.ts";
export * from "./index.ts";
