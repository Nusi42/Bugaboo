import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import type { Attempt } from "./agent-work.ts";
import { isWebAddress } from "./web-address.ts";

type Result = Attempt["results"][number];

/** What tells a result from the others of its attempt. */
function keyOf(result: Result, at: number): string {
  return [at, result.kind, result.label].join("/");
}

/** One result in words: a link the agent made, or a check with its outcome. */
function ResultLine({ result }: { readonly result: Result }) {
  if (result.kind === "check") {
    const failed = result.outcome === "failed";
    return (
      <li {...partOf(part.agentWorkResult, { variants: [failed && "failed"] })}>
        {t(failed ? "agent-work.result-check-failed" : "agent-work.result-check-passed", {
          label: result.label,
        })}
      </li>
    );
  }
  const link = result.link ?? "";
  return (
    <li {...partOf(part.agentWorkResult)}>
      {isWebAddress(link) ? <Link href={link}>{result.label}</Link> : result.label}
    </li>
  );
}

/** What an attempt attached: links and checks. Bugaboo does not run or verify the checks, and says so. */
export function Results({ attempt }: { readonly attempt: Attempt }) {
  if (attempt.results.length === 0) {
    return null;
  }
  const checks = attempt.results.some((result) => result.kind === "check");
  return (
    <div {...partOf(part.agentWorkSection)}>
      <ul {...partOf(part.agentWorkResults)} aria-label={t("agent-work.results")}>
        {attempt.results.map((result, at) => (
          <ResultLine key={keyOf(result, at)} result={result} />
        ))}
      </ul>
      {checks ? <p {...partOf(part.muted)}>{t("agent-work.checks-note")}</p> : null}
    </div>
  );
}
