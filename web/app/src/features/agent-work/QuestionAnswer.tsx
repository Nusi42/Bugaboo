import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  buttonType,
  fieldKind,
  Form,
  partOf,
  part,
  TextField,
} from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { WrittenBy } from "../../shared/WrittenBy.tsx";
import { useAnswerQuestion, type Work } from "./agent-work.ts";

type Question = Work["questions"][number];

/** The words of the agent as a bubble: its name and the mark that an agent wrote them. */
function AgentBubble({ work, text }: { readonly work: Work; readonly text: string }) {
  return (
    <div {...partOf(part.agentWorkBubble)}>
      <span {...partOf(part.agentWorkBubbleHead)}>
        <span>{work.agent_name}</span>
        <WrittenBy mark={{ kind: "agent", agent: work.agent_name }} />
      </span>
      <p {...partOf(part.agentWorkWords)}>{text}</p>
    </div>
  );
}

/** An answered question: what the agent asked, and the answer that joined the brief. */
function Answered({ work, question }: { readonly work: Work; readonly question: Question }) {
  return (
    <div {...partOf(part.agentWorkQuestion)}>
      <AgentBubble work={work} text={question.text} />
      <div {...partOf(part.agentWorkBubble, { variants: ["yours"] })}>
        <p {...partOf(part.agentWorkWords)}>
          {t("agent-work.question-answered", { answer: question.answer ?? "" })}
        </p>
      </div>
    </div>
  );
}

/**
 * A question of the agent with its answer box right there (UX §4.17): the answer joins the brief, so
 * the next attempt and any other agent read it. It works on a phone as it does on a computer.
 */
function Unanswered({ work, question }: { readonly work: Work; readonly question: Question }) {
  const [answer, setAnswer] = useState("");
  const send = useAnswerQuestion();
  const nothingWritten = answer.trim().length === 0;
  return (
    <Form
      onSubmit={() => {
        send.mutate(
          {
            params: { path: { work: work.id } },
            body: { question: question.id, answer, version: work.version },
          },
          {
            onSuccess: () => {
              setAnswer("");
            },
          },
        );
      }}
    >
      <AgentBubble work={work} text={question.text} />
      <TextField
        label={t("agent-work.question-answer")}
        kind={fieldKind.multiline}
        value={answer}
        onChange={setAnswer}
      />
      <p {...partOf(part.muted)}>{t("agent-work.question-joins")}</p>
      {send.isError ? <Alert tone={alertTone.danger}>{messageOf(send.error)}</Alert> : null}
      <div {...partOf(part.agentWorkActions)}>
        <Button
          type={buttonType.submit}
          look={buttonLook.primary}
          isPending={send.isPending}
          isDisabled={nothingWritten}
        >
          {t("agent-work.question-send")}
        </Button>
      </div>
    </Form>
  );
}

/** The questions of the work, oldest first; one that waits for an answer has its box. */
export function Questions({ work }: { readonly work: Work }) {
  return work.questions.map((question) =>
    question.answer === null || question.answer === undefined ? (
      work.open ? (
        <Unanswered key={question.id} work={work} question={question} />
      ) : null
    ) : (
      <Answered key={question.id} work={work} question={question} />
    ),
  );
}
