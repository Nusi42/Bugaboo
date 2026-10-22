import { type Browser, type BrowserContext, expect, test as base } from "@playwright/test";

import { readPorts } from "../../../tooling/ports.ts";
import { type Instance, type Settings, startServer } from "./server.ts";
import { Tolerances } from "./tolerance.ts";

/** The two things of a console message that the fixture reads. */
interface ConsoleLine {
  readonly type: () => string;
  readonly text: () => string;
}

/**
 * What the app wrote to `console.error` or threw without catching, in the test that runs now. The
 * app reports a defect with `console.error` (for instance a message text that is not loaded,
 * ADR-0028), so a test fails when the app does.
 */
const reports: string[] = [];

/**
 * The browser writes a line for every refused request. Tests of refusals expect those, so a
 * line about a 4xx status does not count. Any other line about a request (a 5xx status, a
 * `net::ERR_*`) is a real failure and counts; the one test that expects a 502 (the mail server
 * turned an e-mail away, `mail.rejected`) says so with `tolerate`.
 */
const REFUSED = /^Failed to load resource: the server responded with a status of (4\d\d)/u;

/** The console lines a test expects of a browser that it cuts off the network (see `tolerate`). */
const tolerances = new Tolerances();

/** Collects the defects of every page that a context opens, now and later. */
export function watchApp(context: BrowserContext): void {
  context.on("console", (message: ConsoleLine) => {
    const text = message.text();
    if (message.type() === "error" && !REFUSED.test(text) && !tolerances.expects(context, text)) {
      reports.push(text);
    }
  });
  // An uncaught error is never expected, whatever the network does.
  context.on("weberror", (error) => {
    reports.push(`uncaught: ${error.error().message}`);
  });
}

/**
 * Makes every context that `browser.newContext` opens a watched one (see [`watchApp`]): the
 * default one of a test, and the browsers of their own that a spec opens. Returns the way back.
 */
function watchedBrowser(browser: Browser): { restore: () => void } {
  const open = browser.newContext.bind(browser);
  browser.newContext = async (options) => {
    const context = await open(options);
    watchApp(context);
    return context;
  };
  return {
    restore: () => {
      browser.newContext = open;
    },
  };
}

/** A browser that is cut off the network and brought back (see `tolerate`). */
interface Offline {
  readonly begin: () => Promise<void>;
  readonly end: () => Promise<void>;
}

/** Gets a server into the state a file of tests starts from. */
type Prepare = (server: Instance) => Promise<void>;

/** The one server of a worker. A worker runs the files of tests one after the other. */
interface Servers {
  /**
   * The server for the file: the running one when the file is the same, else a new one with empty
   * data (or a fresh copy of the sample organization), which `prepare` then brings to the state
   * the file starts from.
   */
  readonly forFile: (
    file: string,
    settings: Settings,
    sample: boolean,
    prepare: Prepare,
  ) => Promise<Instance>;
  readonly stop: () => Promise<void>;
}

/** One server per worker: each worker has a port of its own (ADR-0023). */
function servers(parallelIndex: number, label: string): Servers {
  const ports = readPorts();
  const port = ports.e2eWorkersFrom + parallelIndex;
  if (port > ports.e2eWorkersTo) {
    throw new Error("More workers than ports: lower `workers` in the Playwright config.");
  }
  let running: { readonly file: string; readonly instance: Instance } | null = null;
  const stop = async (): Promise<void> => {
    await running?.instance.stop();
    running = null;
  };
  return {
    forFile: async (file, settings, sample, prepare) => {
      if (running?.file === file) {
        return running.instance;
      }
      await stop();
      const instance = await startServer(port, label, settings, sample);
      running = { file, instance };
      await prepare(instance);
      return instance;
    },
    stop,
  };
}

interface Options {
  /** Start-up settings of the server of a file (`test.use({ serverSettings: … })`). */
  serverSettings: Settings;
  /**
   * Whether the server of a file starts on a fresh copy of the sample organization instead of an
   * empty data folder (`test.use({ sample: true })`). The people and their passwords are in
   * `samplePeople()`.
   */
  sample: boolean;
  /**
   * What a file needs in place before its first test (`test.use({ preparation: { run: … } })`).
   * A function inside an object, because Playwright takes a plain function for a fixture.
   */
  preparation: { readonly run: Prepare };
  /**
   * A name for the data of a group of tests (`test.use({ freshData: "dusk" })`): the server of a
   * file starts again, on fresh data, whenever it changes, so each look of a screenshot spec starts
   * from the same issue and no look shows what another one did.
   */
  freshData: string;
}

/**
 * The tests of Bugaboo. A file of tests gets a real Bugaboo server with an empty data folder
 * (`server`), which stays the same for all the tests of the file; they run one after the other.
 * Pages open at its address.
 */
export const test = base.extend<
  Options & {
    server: Instance;
    appReports: string[];
    /**
     * For a test that cuts the network of one browser on purpose (a lock holder who goes offline):
     * the console lines of that context that match `lines` are expected, from `begin` (which cuts
     * the network) until `end` (which brings it back). Lines of any other context, lines while it
     * is online, other lines, and uncaught errors are never let off. With `{ network: "stays-up" }` the
     * network stays up and only the lines are expected between `begin` and `end` (a refusal the test
     * causes on purpose). Only `concurrent/locks.spec.ts`, `mail-server.spec.ts` and `screenshots-notifications.spec.ts` may use it
     * (TEST-CONCURRENT).
     */
    tolerate: (
      context: BrowserContext,
      lines: Readonly<RegExp>,
      mode?: Readonly<{ network: "cut" | "stays-up" }>,
    ) => Offline;
  },
  { pool: Servers }
>({
  pool: [
    async ({ browserName }, provide, workerInfo) => {
      const pool = servers(workerInfo.parallelIndex, browserName);
      await provide(pool);
      await pool.stop();
    },
    { scope: "worker" },
  ],
  serverSettings: [{}, { option: true }],
  sample: [false, { option: true }],
  preparation: [{ run: () => Promise.resolve() }, { option: true }],
  freshData: ["", { option: true }],
  server: async ({ pool, serverSettings, sample, preparation, freshData }, provide, testInfo) => {
    const data = freshData === "" ? testInfo.file : `${testInfo.file}#${freshData}`;
    await provide(await pool.forFile(data, serverSettings, sample, preparation.run));
  },
  tolerate: async ({ browser: _browser }, provide) => {
    await provide((context, lines, mode) => {
      const cut = mode?.network !== "stays-up";
      const network = tolerances.add(context, lines);
      return {
        begin: async () => {
          network.cut();
          if (cut) {
            await context.setOffline(true);
          }
        },
        end: async () => {
          if (cut) {
            await context.setOffline(false);
          }
          // A round trip to every page, so the lines written while it was offline have arrived.
          await Promise.all(context.pages().map((page) => page.evaluate(() => 0)));
          network.restored();
        },
      };
    });
  },
  browser: [
    async ({ browser }, provide) => {
      const watched = watchedBrowser(browser);
      await provide(browser);
      watched.restore();
    },
    { scope: "worker" },
  ],
  appReports: [
    async ({ browser: _browser }, provide) => {
      reports.length = 0;
      tolerances.clear();
      await provide(reports);
      expect(reports.splice(0), "the app reported a defect").toEqual([]);
    },
    { auto: true },
  ],
  contextOptions: async ({ server, browserName }, provide) => {
    // Copying to the clipboard is allowed, as it is for a person who presses the button. WebKit
    // has no such permission to give: it lets a press of the button copy anyway.
    await provide({
      baseURL: server.url,
      permissions: browserName === "chromium" ? ["clipboard-read", "clipboard-write"] : [],
    });
  },
});

export { expect } from "@playwright/test";
