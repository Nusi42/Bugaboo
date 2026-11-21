# Accessibility statement: a model to fill in

An organization that runs Bugaboo for its people, or for the public, may need to publish an accessibility statement:
a public body in the European Union must (Directive (EU) 2016/2102), and a customer may ask for one. This page is a
model in the parts of the EU model statement. Copy it, fill in every part in square brackets, and publish it
where people find it, for example from a page of your intranet.

What Bugaboo itself meets is written in the [EN 301 549 report](../docs/accessibility/en-301-549.md), criterion by
criterion. The model below says the same in short; when you run a version of Bugaboo with a newer report, take
the parts that are not met from that report.

---

## Accessibility statement for [name of the organization]'s Bugaboo

[Name of the organization] is committed to an instance of Bugaboo that everyone can use, in accordance with
[national law, for example the BFSG or the BITV 2.0 in Germany] transposing Directive (EU) 2016/2102 of the European
Parliament and of the Council.

This accessibility statement applies to [the address of your Bugaboo, for example `https://bugaboo.example.org`],
version [the version shown on *About*].

### Compliance status

This website is **partially compliant** with EN 301 549 V3.2.1 and with WCAG 2.2 at level AA, due to the
non-compliances listed below.

### Non-accessible content

The content listed below is not accessible for the following reasons.

**Non-compliance with the accessibility requirements:**

- Bugaboo signs a person out after a time without activity that the administrator sets (30 minutes at the start),
  without a warning before. Text that was being written is kept on the device and offered again after signing in.
  [If you set *Sign out after no activity* to 1200 minutes or more, delete this point: a limit of 20 hours or more
  meets the criterion.] (WCAG 2.2.1)
- Text that people write in descriptions and comments is shown in the language of the page; a passage in another
  language cannot be marked as such. (WCAG 3.1.2)
- When an image is added to a description or a comment, Bugaboo does not ask for a text that describes it. People
  can write one with the image's mark, but they are not reminded. (EN 301 549 11.8.4)
- Every control of every page is reached with the keyboard, but that the main task of a page can be done with the
  keyboard alone is tested only on some pages: the issue list, the timesheet and the timer, the timeline, *Data
  exports*, the theme editor, the assistant, *My agents* and some windows. (WCAG 2.1.1)
- Bugaboo passed automatic checks (contrast and the other rules of axe in every color theme, the keyboard, the
  sizes of targets, reflow, pages enlarged to 200 % and text spacing), but no person has yet tested it with a
  screen reader. Whether names, the order of reading and announcements are clear when heard is not tested. (WCAG 1.1.1, 1.3.1, 1.3.2,
  4.1.2, 4.1.3)
- [Anything your organization added: own themes, own fields with long names, files and pictures that people
  upload. The theme editor says which colors of an own theme are hard to read; files that people upload are their own
  content.]

**Disproportionate burden:** [Not applicable, or what you hold to be one, and why.]

**Content not within the scope of the applicable legislation:** [For example: files that people upload, such as PDF
documents and pictures, which are their own content and not written by the organization.]

### Preparation of this accessibility statement

This statement was prepared on [date]. It was last reviewed on [date].

The statement is based on [a self-assessment by the organization / an assessment by a third party, name], together
with the accessibility report of the version of Bugaboo named above, which was carried out with automatic tests on
every page (in four color themes, on a computer and a phone, at 320 pixels wide, enlarged to 200 % and with wide
text spacing) and a walk of every page with the keyboard. No person has tested Bugaboo with a screen reader yet [unless
you did: then name the date and the screen reader].

### Feedback and contact information

If you find a part of Bugaboo that you cannot use, or you need information in another form, tell us:

- E-mail: [the address of the person or team who answers]
- Phone: [number, with the hours it is answered]
- Post: [address]

We answer within [number] days.

### Enforcement procedure

If you are not satisfied with our answer, you can contact [the enforcement body of your country, for example in
Germany the *Schlichtungsstelle nach dem Behindertengleichstellungsgesetz*, with its postal and web address].

---

## What to change when Bugaboo changes

Each version of Bugaboo comes with its report. When you upgrade, read the list of what is not met in the new report,
change the list above, and write the new date under *Preparation of this accessibility statement*.
