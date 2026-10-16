/**
 * The connections a run of the browser tests leaves behind, and the limit that guards them
 * (TEST-CONNECTIONS, M4 WP-03). A connection that is closed stays in TIME_WAIT for about two
 * minutes, and one that the other side closed but this side did not stays in CLOSE_WAIT; on
 * Windows a computer that has too many of them answers `net::ERR_NO_BUFFER_SPACE`. The pure part
 * is here (reading a list of connections, judging); the reporter that asks the computer is
 * `connection-guard.ts`.
 */

/**
 * The most connections in TIME_WAIT and CLOSE_WAIT that the servers of the tests (the ports of
 * their workers) may have at the same moment during a run. A run peaked at 1,248 after M4 WP-03
 * (1,989 before it; architecture §11), so this is about twice that. Windows gives out 16,384
 * ports for connections, so five work folders that each reach it at the same time stay under that.
 * Not to be raised to make a run pass.
 */
export const MOST_LEFTOVER_CONNECTIONS = 2_500;

/** How often the computer is asked for its connections. */
export const SAMPLE_EVERY_MS = 1000;

/** What the guard counts of one worker's server. */
export interface Counts {
  /** Connections of the tests that are open (counted at the end that connects to the server). */
  readonly established: number;
  /** Closed by the side of the tests or the server and not yet let go by the system. */
  readonly timeWait: number;
  /** Closed by the other side and not yet closed here. */
  readonly closeWait: number;
}

/** One look at the computer: the counts of every worker's port and their sum. */
export interface Sample {
  readonly total: Counts;
  readonly byPort: Readonly<Record<number, Counts>>;
}

const NONE: Counts = { established: 0, timeWait: 0, closeWait: 0 };

/**
 * The states that the guard reads, as the systems spell them (`TIME_WAIT`, `TIME-WAIT`, `ESTAB`,
 * `TimeWait` of PowerShell). `netstat` of Windows prints them in the language of the system, which
 * is why the reporter asks PowerShell there.
 */
function stateOf(word: string): keyof Counts | undefined {
  switch (word.toUpperCase().replaceAll(/[-_]/gu, "")) {
    case "ESTABLISHED":
    case "ESTAB":
      return "established";
    case "TIMEWAIT":
      return "timeWait";
    case "CLOSEWAIT":
      return "closeWait";
    default:
      return undefined;
  }
}

const PORT_AT_END = /:(\d+)$/u;

function portOf(address: string): number | undefined {
  const found = PORT_AT_END.exec(address);
  return found?.[1] === undefined ? undefined : Number(found[1]);
}

function added(counts: Counts | undefined, state: keyof Counts): Counts {
  const before = counts ?? NONE;
  return { ...before, [state]: before[state] + 1 };
}

/**
 * Reads the list that `ss -tan` or `netstat -ant` (Linux) or the PowerShell line of the reporter (Windows) prints, and
 * counts the connections of the workers' servers, which listen on ports `from` to `to`. A line is
 * the local address, the remote address and a state, in that order. A connection counts for the
 * server whose port is one of its two ends; an open one is counted only at the end that connects,
 * so that it is not counted twice (both ends of a connection on this computer are in the list).
 */
export function countConnections(listing: string, from: number, to: number): Sample {
  const byPort: Record<number, Counts> = {};
  let total = NONE;
  const inRange = (port: number | undefined): port is number =>
    port !== undefined && port >= from && port <= to;
  for (const line of listing.split("\n")) {
    const words = line.trim().split(/\s+/u);
    const state = words.map((word) => stateOf(word)).find((found) => found !== undefined);
    const ends = words.filter((word) => PORT_AT_END.test(word)).map((word) => portOf(word));
    const [local, remote] = ends;
    if (state === undefined || ends.length < 2) {
      continue;
    }
    const server = inRange(remote) ? remote : inRange(local) ? local : undefined;
    // An open connection is two lines on this computer; the one whose remote end is the server
    // is the one of the tests.
    if (server === undefined || (state === "established" && server !== remote)) {
      continue;
    }
    byPort[server] = added(byPort[server], state);
    total = added(total, state);
  }
  return { total, byPort };
}

/** What counts against the limit: connections that were closed and are not let go yet. */
export function leftOver(counts: Counts): number {
  return counts.timeWait + counts.closeWait;
}

/** Where the most connections were, and while which spec ran there. */
interface Peak {
  readonly left: number;
  readonly counts: Counts;
  readonly port: number | undefined;
  readonly spec: string;
}

/** What the run reached, kept look by look; judged at the end. */
export class ConnectionGuard {
  private peak: Peak = { left: 0, counts: NONE, port: undefined, spec: "" };
  private mostOpen = 0;
  private looks = 0;

  private readonly limit: number;

  public constructor(limit = MOST_LEFTOVER_CONNECTIONS) {
    this.limit = limit;
  }

  /** Takes one look; `specs` says, for each worker port, which spec its worker ran last. */
  public record(sample: Sample, specs: Readonly<Record<number, string>>): void {
    this.looks += 1;
    this.mostOpen = Math.max(this.mostOpen, sample.total.established);
    const left = leftOver(sample.total);
    if (left <= this.peak.left) {
      return;
    }
    let worst: number | undefined;
    for (const [name, counts] of Object.entries(sample.byPort)) {
      if (worst === undefined || leftOver(counts) > leftOver(sample.byPort[worst] ?? NONE)) {
        worst = Number(name);
      }
    }
    this.peak = {
      left,
      counts: sample.total,
      port: worst,
      spec: worst === undefined ? "" : (specs[worst] ?? ""),
    };
  }

  /** What was reached, in one line for the end of the run. */
  public summary(): string {
    const { counts, port, spec } = this.peak;
    const where =
      port === undefined
        ? ""
        : ` Most at the peak: the server on port ${String(port)} (${spec === "" ? "no spec yet" : spec}).`;
    return (
      `Connections of the tests: at most ${String(this.peak.left)} left over at once ` +
      `(${String(counts.timeWait)} in TIME_WAIT, ${String(counts.closeWait)} in CLOSE_WAIT), ` +
      `at most ${String(this.mostOpen)} open, in ${String(this.looks)} looks; ` +
      `the limit is ${String(this.limit)}.${where}`
    );
  }

  /** The sentence that fails the run when the peak is over the limit, else `undefined`. */
  public verdict(): string | undefined {
    if (this.looks === 0) {
      return "The connections of the tests could not be counted, so nobody knows whether the run uses up the ports of the computer. Is `netstat` (Windows) or `ss` (Linux) installed?";
    }
    if (this.peak.left <= this.limit) {
      return undefined;
    }
    const spec = this.peak.spec === "" ? "no spec yet" : this.peak.spec;
    const where =
      this.peak.port === undefined
        ? ""
        : ` Most of them belong to the server on port ${String(this.peak.port)}, while its worker ran ${spec}.`;
    return (
      `The browser tests left ${String(this.peak.left)} connections in TIME_WAIT and CLOSE_WAIT at once ` +
      `(${String(this.peak.counts.timeWait)} and ${String(this.peak.counts.closeWait)}); the limit is ${String(this.limit)}. ` +
      `A computer with too many of them answers net::ERR_NO_BUFFER_SPACE.${where} ` +
      "Make the spec use fewer connections (one session per person, no probes in a loop); do not raise the limit."
    );
  }
}
