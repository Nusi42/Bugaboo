# What you can write

Every text box for a description, a comment or long text has two tabs, "Write" and "Preview", and a row of buttons above the text. The preview shows exactly what others will see: it is made by the same program that shows the text on the server.

## The buttons

Select some text and press a button, or press it without a selection and type between the marks it writes.

| Button | What it writes | What you get |
|--------|----------------|--------------|
| Bold | `**text**` | **text** |
| Italic | `*text*` | *text* |
| List | `-` and a blank before each line | a list with bullets |
| Link | `[text](https://)` | a web address you can press |
| Code | `` `text` `` (a fence of three marks for several lines) | text in a box, as typed |
| Quote | `>` and a blank before each line | an indented quote |

Press a button a second time to take the marks off again. You can also type the marks yourself.

## More marks

- A line that starts with `#`, `##` or `###` is a heading.
- A line that starts with `1.` is a numbered list.
- `~~text~~` crosses text out.
- A line with only `---` draws a line across.
- A table is written with bars: a line of headings, a line of `---` under each, then the rows.

## Issue IDs and mentions

An issue ID such as `WEB-142` or `WEB-K7Q2X` and a mention such as `@ana` are recognized in the text and marked. They are not recognized inside code, or inside a web address.

## Web addresses and images

- A web address starts with `https://`, `http://` or `mailto:`, or is a page of this server such as `/p/WEB`. It opens in a new tab. Other kinds of address, such as the ones that start with `javascript:` or `data:`, are not made into links; the words stay as text.
- An image shows when it is a picture of this server. Pictures from other hosts do not show yet: the list that an administrator keeps under *Organization settings › System › Security* is there, and showing them follows in a later version. For any other image, the words and the address stay, as a web address you can press.

## What does not work, on purpose

Raw HTML is shown as text. If you paste `<script>alert(1)</script>`, you and everybody else see those characters, and nothing runs. The same goes for styles, forms and frames. This keeps everybody safe, also when a text comes from someone else.

Very long texts are cut, quotes inside quotes keep their shape only a limited number of levels down, and a text with a very great number of issue IDs and mentions marks only the first ones.
