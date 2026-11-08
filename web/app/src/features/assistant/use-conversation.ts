import { turnsSent, useTurns, type Turn } from "./conversation.ts";
import { useAsk, type AssistantAnswer } from "./assistant.ts";
import type { LookingAtItem } from "./looking-at.ts";

/** The conversation of the open panel: the turns, the question that waits for its answer, and how to ask. */
export interface Conversation {
  readonly turns: readonly Turn[];
  /** The question that has been sent and is not answered yet. */
  readonly pending: string | undefined;
  readonly error: unknown;
  readonly ask: (text: string) => void;
}

/**
 * Asks the assistant about what the panel looks at, with the conversation so far: the server keeps
 * none of it (M7-Q6). A question joins the turns when its answer comes; one that fails stays out, so
 * it can be sent again without standing twice.
 */
export function useConversation(items: readonly LookingAtItem[]): Conversation {
  const { turns, addQuestion, addAnswer } = useTurns();
  const asking = useAsk();
  return {
    turns,
    pending: asking.isPending ? asking.variables.body.text : undefined,
    error: asking.error,
    ask: (text) => {
      asking.mutate(
        {
          body: {
            text,
            looking_at: items.map((each) => ({ kind: each.kind, value: each.value })),
            turns: [...turnsSent(turns)],
          },
        },
        {
          onSuccess: (answer: AssistantAnswer) => {
            addQuestion(text);
            addAnswer(answer);
          },
        },
      );
    },
  };
}
