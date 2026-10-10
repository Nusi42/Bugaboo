# Rules

A field can have rules. The page of the field has a **Rules** section for the kinds that can have them.

- **Required.** The field has to be filled in. A person who cannot change the field is not asked to fill it in, and a field with a starting value is filled in by itself. A required field cannot be emptied.
- **Lowest and highest value** for numbers, decimals and lengths of time.
- **Fewest and most characters** for texts and web addresses, and **fewest and most values** for *Pick several* and *People*.
- **Pattern** for texts: the whole text has to match. Patterns are checked by a method that stays fast whatever the text, and a pattern is at most 200 characters long. A pattern that looks ahead or behind, or that repeats an earlier part of the text, is refused when you save it.

## What you are told

The same rules are sent to your browser, so a value that breaks one is marked as soon as you type it. The server decides in the end, and says in a sentence which rule the value breaks: *This field has to be filled in*, *That is shorter than this field allows*, *That is higher than this field allows*, *That does not have the form this field asks for*, *That choice is not offered by this field*. Nothing is saved when a value is refused.

A field is offered only where it is used. A value for a field that is retired, or not used in this project or for this issue type, is refused with *This field is not used here*.
