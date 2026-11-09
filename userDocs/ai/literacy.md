# AI in Bugaboo: a short guide for everybody

Bugaboo has three kinds of AI help. This guide says in plain words what each one can and cannot do, where your data goes, how to check what an AI wrote, and what Bugaboo never lets AI do. It is for the people who use Bugaboo and for the administrators who switch AI on. Many organizations must ensure that the people who work with AI understand it; this guide is a part of that, and not legal advice.

AI is **off** until an administrator switches it on.

## The three kinds

- **The assistant** is the helper in the panel beside the page. It works for you, in your browser. It reads what you are looking at, answers, and **proposes** changes. Nothing is saved until you press **Apply**.
- **An AI app** is a tool such as Claude Code that you let into your account. It works for you, through the account you allowed it for. It looks at, and if you allowed it, changes what you may look at and change.
- **An agent** is a helper that works on its own and reports through its own key. It works for the one person who answers for it. It works on the issues you hand over, or on what a standing rule finds.

None of them is a person, and none of them can do more than the person behind it.

In this guide, a **tool** is what the assistant, an AI app or an agent uses to look something up in Bugaboo or to change it.

## What AI can do

- **The assistant** can summarize an issue, answer a question about the issues you are looking at, and propose a change, for example a priority. A proposal shows the value before and after, and a proposal that names people shows how many. It stays a proposal until you press **Apply**, and it runs out if you wait too long. After **Apply** you can undo it.
- **An AI app** works as you. It sees what you see and changes what you may change, and nothing more. You allow it once. The app asks for *See only* or for *See and change*; you can allow less than it asks, and you can limit it to some projects.
- **An agent** has its own key and works on **hand-overs** and **standing rules**. A hand-over gives it a few issues with a note, a budget and an end. A standing rule gives it what a saved search finds, by proposals only unless you name a few fields it may change. Everything it writes says *Written by an agent*. It changes an issue only while one of its attempts runs on it; on an issue that only a standing rule found, it reads and proposes and changes nothing.

The numbers (how long a question may be, how many issues one hand-over holds, how long a rule lasts) are on the page [Numbers of the assistant, AI apps and agents](numbers.md).

## What AI cannot do

- It sees **only what the person behind it may see**. A field that is hidden from you is hidden from your assistant, your apps and your agents.
- It never changes **who has which role**, a person's settings, a password or a key.
- An agent can never be given a field that holds **people** (*Assigned to*, *Followers*, *Reported by*). The assistant and AI apps are told never to choose a person for you. A proposal that names people shows how many before you press **Apply**; look at the issue afterwards to see who they are.
- An agent is **never assigned** to an issue and never holds a role.
- An **agent** cannot book **time**. What it reports about its own time and cost stays on its attempt and is never a time entry. An AI app that you allowed to change things books time only as you, like you would.
- An agent cannot go on when its person **loses the right to have agents**: its work stops.

## Where your data goes

Only the assistant sends text to an **AI provider**. An administrator chooses the provider under *Organization settings › Connections › AI* (**Set up the assistant**): *Anthropic*, *Other provider or own server*, or *A model on your own server*. The part *What goes where* on the same page shows the provider and its country.

- For a provider on the internet, an administrator must **confirm that a data processing agreement is in place** before AI can be switched on. Bugaboo keeps who confirmed it and when. A model on your own servers needs none, because nothing leaves your organization.
- What goes to the provider is the question, the conversation and what you are looking at, and only what you may see. **People** go to a provider on the internet as their role ("a Team member"), not by name, unless an administrator switches *Send names of people* on. With a model on your own server, nothing leaves the organization and people are shown as you see them.
- **Never sent to AI:** an administrator can mark a field, or a whole project, so that it is kept from the assistant, from AI apps and from agents, even from people who may read it. Use it for what must never leave: a customer's name, a price, a project under secrecy.
- **AI can be switched on or off for each project.** A project can also switch **Agents** off.
- AI apps and agents do not send your data to a provider through Bugaboo. They run outside Bugaboo and read what you let them read, with people's names as the person behind them sees them. What they do with it is their own business, so allow only apps and agents you trust. Bugaboo cannot check the name an app gives itself.
- **The agreement Bugaboo asks for covers only the provider of the assistant.** An AI app (such as Claude Code, which sends to Anthropic) and an agent send to their own vendor, under the agreement your organization has with that vendor, if it has one. Bugaboo is not a part of it and cannot see it. Check it before you allow an app or create an agent.
- Bugaboo keeps a log of the tools an AI used: who, which tool, how big the answer was and when, never the content. It is kept for the number of days on the page of numbers.

## How to check what an AI wrote

1. **Look at the mark.** Every text an AI wrote carries a mark beside it: *Written with the assistant*, *Written by an AI app* (with the name of the app) or *Written by an agent* (with its name). The mark is stored with the text, shown in the history, and given in the API and every export. If you edit such a text yourself, the mark stays and the history says who changed it.
2. **Read the history.** A change by an AI says so: "Fixer, agent of Ada Brandt", or "Ada Brandt via Claude Code".
3. **Look at the run.** Everything one session of an AI app or one attempt of an agent changed is one **run**, with its list of changes. An agent can change an issue handed over to it only while an attempt of it runs there, so what it changes there belongs to the run of that attempt.
4. **Undo the run.** **Undo the run** puts the old values back in one step. It first shows what goes back and what is skipped, for example "Ben changed it afterwards, so that change stays." It never deletes history: it writes new changes. For an agent, **Throw away** at the review does the same for an attempt.
5. **Do not trust, check.** An AI can be wrong in a way that sounds right. A result an agent calls *passed* is its own word: Bugaboo does not run its checks.

## What Bugaboo never lets AI do

These are rules of the product, not settings.

- AI never **decides about people**: it never suggests who should work on something, never ranks people. It does not choose who gets a task: it sets a person only when you name that person and ask for it, and then only as a proposal you apply.
- AI never **watches or evaluates** people: not their performance, not their behaviour, and not their working hours. No tool of the assistant, of apps or of agents reads anybody's work days.
- AI never does **emotion or mood analysis** of what people write.
- AI never acts **in secret**: everything it writes is marked, and every change it makes is in the history and in a run.

## For administrators: a short list

1. **Set up the assistant** and confirm the agreement for a provider on the internet. See [Connected apps](../access-keys/connected-apps.md) for what apps get.
2. Decide which **projects** have AI, and mark what is **Never sent to AI**.
3. Decide whether **Agents** are on (*Organization settings › Connections › Agents*, and the switch *Agents in this project*), and the most an agent may do.
4. Tell your people where this guide is (they reach it from *Set up the assistant* and from the assistant), and ask them to check what an AI wrote before they rely on it.
5. You can **stop** any agent and **take back** any app.

More: [Agents: hand-overs and standing rules](../agents/README.md), [Connected apps](../access-keys/connected-apps.md), [The assistant](assistant.md), [AI apps](ai-apps.md).
