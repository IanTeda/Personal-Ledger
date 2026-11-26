# The text-field model is the Presenter's, not the View's

Paste, copy, cut and keyboard selection in the Desktop's text boxes ([DESKTOP: clipboard and text selection in text boxes](https://github.com/IanTeda/Personal-Ledger/issues/1460)) need a caret that can stand anywhere in the text and a selection. Until now `lib-viper` said a text field's caret was the View's, as widget mechanics: the caret was always drawn after the last character, and each Presenter changed its text one `DialogInput::Char` (`push`) or `Backspace` (`pop`) at a time, about 45 of them each in their own way. [DESKTOP: where do a text box's caret, selection and clipboard live?](https://github.com/IanTeda/Personal-Ledger/issues/1463) decided the caret and selection move into the toolkit-free model.

- **One deep module, `TextField`, in `lib-viper`.** It holds a field's text, its caret and its selection, and takes one input, `edit(TextEdit)`: type a character, Backspace, Delete, paste a string, ←/→/Home/End (each optionally extending the selection), select all, and cut. A Presenter holds one `TextField` per text field and hands it the focused field's edits; no Presenter edits text itself.
- **`DialogInput::Text(TextEdit)` replaces `Char` and `Backspace`.** A Presenter still matches on which field is focused, then passes the edit through.
- **The paste rule is the model's.** A `TextField` is single-line or multi-line. A paste into a single-line field drops line breaks and tabs and trims spaces from both ends; a multi-line field keeps its newlines.
- **Caret steps are grapheme clusters** (`unicode-segmentation`), so an accented letter or an emoji built from several code points moves and deletes as one.
- **The clipboard stays in the bin.** For paste, the bin reads the clipboard and sends `TextEdit::Paste(text)`. For copy, it reads the selected text from the view model and writes the clipboard with no Presenter round trip. For cut, it does the same, then sends `TextEdit::Cut`, which deletes the selection. `lib-viper` never touches the clipboard and Presenters report no new effects.
- **`TextFieldView` carries the caret and the selection**, and the View draws both.
- **A hidden field is a kind of `TextField`.** A secret field holds its text as a `SecretString`, carries a shown/hidden flag that a reveal edit flips, refuses cut while hidden, and puts only its length in the view model while hidden, so neither the View nor a copy ever sees the text. Its look and the eye button are the View's.

## Considered Options

- **Keep the caret and selection in the View**, sending the Presenter only finished edits. Rejected: a field's state would be split, with the selection in the View and the text in the Presenter, and cut and paste at the caret would need both.
- **Adopt gpui-component's `Input`.** Rejected: it owns the text in a gpui Entity beside the Presenters that already own it, giving two key paths and two focus models. Its masked mode copies the plaintext, its eye reveals only while held and has no Tab stop, and its paste only strips `\n`. It would have brought IME, mouse selection and undo, which this design gives up for now.

## Consequences

- Every editing rule (cleaning a paste, refusing a hidden field's cut, grapheme steps) is written and tested once, without gpui.
- Each Presenter is touched once to hold `TextField`s. Until a Presenter moves over, it matches `Text(TextEdit::Insert)` and `Text(TextEdit::Backspace)` and ignores the other edits, so its boxes type as before but don't paste.
- IME composition, mouse selection and undo are not provided. Mouse selection is later work on the map; undo is out of its scope.
