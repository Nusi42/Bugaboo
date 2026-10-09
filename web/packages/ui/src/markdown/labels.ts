/** The words of the editor, already in the language of the person. */
export interface MarkdownEditorLabels {
  readonly write: string;
  readonly preview: string;
  /** The name of the group of formatting buttons for screen readers. */
  readonly toolbar: string;
  readonly bold: string;
  readonly italic: string;
  readonly list: string;
  readonly link: string;
  readonly code: string;
  readonly quote: string;
  /** The text of a new link until the person types its own. */
  readonly linkText: string;
  /** Said in the preview while the text is empty. */
  readonly previewEmpty: string;
  /** Said when the preview could not be loaded. */
  readonly previewFailed: string;
}
