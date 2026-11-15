// The texts of this area come along with its chunk (UX-07).
import "@bugaboo/messages/parts/list-editor";

// The editor of a list of things (projects, labels, issue types, kinds of relation, shortcuts): only the pages that
// edit such a list load it, so the first page stays small (coding-standards section 6).
export { ListEditor, type ListEditorProps, type ListRow } from "./ListEditor.tsx";
