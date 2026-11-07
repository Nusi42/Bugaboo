# WP-10 — The assistant panel and setting up AI on screen

**Wave 3 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** AI-10, AI-05 (on screen), AI-02, AI-03, PRIV-08, COMP-08 (on screen). UX design §4.12, §5.1, §5.3, the pictures WP-01 draws; ADR-0013, ADR-0028 (message parts).

## Goal

The assistant is one click away wherever a person is, and always says that it is AI, what it looks at, where the answers come from and which fields it never gets. A proposed change shows the value before and after, and nothing is saved until *Apply*. When AI is off, every trace of the assistant is gone, and administrators see one *Set up the assistant* item. Setting up AI walks an administrator through the provider, the agreement and *What goes where*.

## Owns

`web/app/src/features/assistant/**`, `web/app/src/features/settings/ai*` (*Organization settings › Connections › AI* and the project's switch), the *Assistant* button of the top bar in `features/shell`, *Never sent to AI* on the *Fields* page and in the project settings, the theme rules of these parts, the user docs' screens of the assistant (the text comes with WP-13).

## What is decided (UX §4.12, M7-Q2, Q5, Q6)

- **The panel** opens from *Assistant* in the top bar (and from *Quick search*), as a side panel on a computer and a full sheet on a phone. On top: *Looking at:* with each item removable (the issue, the list, the project); *"Answers come from Claude (Anthropic). Your organization chose this."* or *"Answers come from a model on your own server."*; *Never sent to AI:* with the fields. Below: the turns; the box to ask.
- **Every answer** is labelled *Assistant* and written by the server's Markdown (no outside pictures, links in full). A **proposed change** is a card: the issue, the field, before and after, the sentence *"Nothing is saved until you apply it."*, *Apply* and *Discard*. After *Apply*, the card says what was saved and offers *Undo* for 10 seconds, like any change.
- **The conversation** lives in the open panel only (M7-Q6): closing it asks first when it has turns.
- **No buttons of AI-12 to AI-14** in M7 (M7-Q2): no sparkle buttons beside fields.
- **When AI is off** at the place, the button, the panel and *Quick search* entries are gone; an administrator sees *Set up the assistant* in *Organization settings*.
- ***Organization settings › Connections › AI*:** *Set up the assistant* (provider, model, address for a local model, the key, *Try the provider*), the agreement step for an outside provider (the confirmation sentence, who and when), the model's information before switching on (COMP-08), *What goes where*, *Send names of people* (if M7-Q5 is taken), the apps that may sign in (M7-Q4), the link to the AI literacy guide (COMP-11). The project settings have the switch *AI in this project* (follows the one above).
- ***Never sent to AI*** is a switch on a field's page (*Fields*) and in the project settings, with a sentence that says it holds even for people who can read the field.
- **A lazy area** with its own message part (ADR-0028): the first page does not grow beyond the button.

## Tasks

1. **The panel** on computer and phone, with the keyboard (open, ask, move between cards, *Apply*), screen readers told when an answer arrives.
2. **The proposal card** made once, used for every kind of tool (WP-11 shows the proposals of standing rules with it).
3. ***Organization settings › Connections › AI*** and the project switch, through the one settings form.
4. ***Never sent to AI*** on fields and projects.
5. **Pictures** of every changed screen in four themes on computer and phone, looked at against UX §1.1 and §7 before the push; the report names them.

## Acceptance tests (write first)

- `ai_10_the_assistant_opens_from_the_top_bar_and_shows_what_it_looks_at`.
- `ai_10_removing_an_item_from_looking_at_leaves_it_out_of_the_question`.
- `ai_05_a_proposed_change_shows_before_and_after_and_saves_only_after_apply`.
- `ai_05_discard_saves_nothing`.
- `comp_08_every_answer_is_labelled_as_the_assistant`.
- `ai_02_with_ai_off_there_is_no_assistant_button_and_administrators_see_set_up_the_assistant`.
- `priv_08_setting_up_an_outside_provider_asks_for_the_agreement`.
- `ai_03_never_sent_to_ai_on_a_field_shows_in_the_panel`.
- TEST-A11Y on every changed screen; the keyboard alone asks, applies and discards.

## Checks

TEST-A11Y, THEME-ONLY, STY-PARTS, STY-BOLD, STY-TOKENS, TXT-TERMS, TXT-CATALOG, ARCH-WEB-API, ARCH-WEB-FEATURES, PERF-BUNDLE.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: one proposal card, the provider's name and country from the server's *What goes where* (never written in the browser), the settings through the one settings form. A UX reviewer compares every picture with UX §4.12, §1.1 and §7. A words reviewer reads every new message (the AI is always *the assistant*).

## Depends on

Wave 2 merged (WP-06; WP-02 and WP-03 from wave 1). Runs beside WP-09, WP-11, WP-12; merges second in wave 3, after WP-09, because WP-11 takes the proposal card from it.
