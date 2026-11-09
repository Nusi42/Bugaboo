# The assistant

The **assistant** is the AI helper beside the page. It is not an agent: it works for you while you use it, it only **proposes**, and it never does anything on its own. Read [the AI guide](literacy.md) first; it is also in the app, linked in the panel and on *Set up the assistant*.

## Opening it

Press **Assistant** in the header of a page. The panel opens beside the page. At its top, **Looking at** says what the assistant may read for this question: the issue, the list or the project you have open. Each has **Leave out**, and the assistant then does not see it. The panel also says where the answers come from (Claude, a provider on the internet, or a model on your own server), whether people are sent by name or as their role, and what is marked *Never sent to AI*.

If AI is off for the organization or for the project, the panel says "AI is off here" and nothing is sent anywhere.

Closing the assistant ends the conversation. It is not stored anywhere.

## Asking

Type a question and press **Ask**. For example "Summarize WEB-142". What the assistant writes in the panel is labelled **Assistant** and is not stored. A text it helps you save, and a change you apply, is marked **Written with the assistant**.

- The assistant looks things up in Bugaboo with the same rights as you. What is hidden from you is hidden from it, and what is marked *Never sent to AI* is kept from it.
- A question, a conversation and one answer have limits; they are on the page [Numbers of the assistant, AI apps and agents](numbers.md).

## Proposals and Apply

When the assistant wants to change something, it shows a **proposed change**: what changes, the value before and after, and for which issue. Nothing is saved yet.

- **Apply** saves the change, as you. The history says the change was done with the assistant.
- **Discard** drops it; nothing is saved.
- A proposal that waits too long runs out. Ask again.
- After **Apply**, the card offers **Undo**, which puts the old value back. If the value changed since, it is not taken back and the card says so.

## For administrators: setting it up

*Organization settings › Connections › AI*, **Set up the assistant**:

1. Choose the **provider** and the model: *Anthropic*, *Other provider or own server* (enter its address), or *A model on your own server*.
2. Enter the **key** of the provider (stored encrypted; never shown again). Changing the provider or the address asks for your password again, and a new provider never gets the key of the old one.
3. For a provider on the internet, **confirm the agreement**: that a data processing agreement is in place. Bugaboo keeps who confirmed it and when. Changing the provider takes the confirmation away.
4. Press **Try the provider**. It makes one short call and says whether the provider answered.
5. Switch AI **on**. Then choose, for each project, whether AI is on there.

Mark what is **Never sent to AI** on the same page: built-in fields, own fields and whole projects. *Send names of people* stays off unless you decide otherwise.
