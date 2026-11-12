//! The device settings form (#1159): what a This device page's fields share, so a page declares
//! its fields ([`FieldDecl`]) and keeps only what is its own. The form owns the keyboard's focus
//! by named field, stepping a field's choices, the one open dropdown list, the write to
//! [`DeviceConfiguration`] and its settling onto the status line, the shadowed note, the legend
//! and each field as drawn ([`FieldView`]). Toolkit-free, so the TUI can reuse it (#805).
//!
//! The form takes a [`FormInput`] and answers a [`FormAnswer`] rather than calling a router, so
//! each page matches an answer to its own router call. A field shows a change only once it is
//! saved: a shadowed key or a refused write leaves it as it was and says why on the status line.

use std::rc::Rc;

use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_list_cursor::Step;
use lib_toast::{Notice, Notify};
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP, SELECT};
use lib_viper::view::{Choice, Hint, SelectList};

use super::device::shadowed_note;
use crate::keymap::{NEXT_CHOICE, PREVIOUS_CHOICE};
use crate::msg;
use crate::protocols::{DeviceAnswer, DeviceConfiguration, DeviceRefusal, DeviceSnapshot};

/// A field's value: one its key holds, none (the key taken out of the file, as the Locale's
/// **System**), or a remembered checkbox's, which no key holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Set(ConfigValue),
    /// No value: choosing it removes the key from the file.
    Unset,
    /// A [`FieldKind::Remembered`] checkbox's state, which the Client keeps, not the file.
    Flag(bool),
}

/// How a field is drawn and which keys it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presentation {
    /// `h`/`l` step its choices, stopping at either end.
    Segmented,
    /// `h`/`l` step its choices; `enter` opens its list, whose keys are `j`/`k`, `enter` and `esc`.
    Dropdown,
    /// Two choices, unticked then ticked: `h` clears, `l` ticks, `enter` and `space` toggle.
    Checkbox,
    /// No choices: `enter` presses it, answered as [`FormAnswer::Activated`].
    Action,
}

/// Where a field's change goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Written to the device and settled.
    Configuration,
    /// Not Configuration: nothing is written, and a change is answered to the page as
    /// [`FormAnswer::Changed`].
    Remembered,
    /// Written and settled, but `j`/`k` skip it: a click reaches it.
    NotInWalk,
}

/// A field's own note from its value and the source above the file that sets its key, if any:
/// the Locale's next-start and override note, the Data folder's "overridden, in effect is X".
pub(crate) type OwnNote = Rc<dyn Fn(&FieldValue, Option<&str>) -> Option<String>>;

/// One field as a page declares it.
#[derive(Clone)]
pub(crate) struct FieldDecl<F> {
    field: F,
    key: Option<ConfigKey>,
    label: String,
    choices: Vec<Choice<FieldValue>>,
    presentation: Presentation,
    kind: FieldKind,
    step_from: Option<FieldValue>,
    change_hint: Option<String>,
    note: Option<String>,
    own_note: Option<OwnNote>,
    when_unset: FieldValue,
    read: fn(&ConfigValue) -> FieldValue,
}

impl<F> FieldDecl<F> {
    /// A Configuration field over `key`, showing what the device reads of it.
    pub fn configuration(
        field: F,
        key: ConfigKey,
        label: String,
        presentation: Presentation,
        choices: Vec<Choice<FieldValue>>,
    ) -> Self {
        Self {
            field,
            key: Some(key),
            label,
            choices,
            presentation,
            kind: FieldKind::Configuration,
            step_from: None,
            change_hint: None,
            note: None,
            own_note: None,
            when_unset: FieldValue::Unset,
            read: |value| FieldValue::Set(value.clone()),
        }
    }

    /// A [`FieldKind::Remembered`] field holding `value`, which the Client hands the page.
    pub fn remembered(
        field: F,
        label: String,
        presentation: Presentation,
        choices: Vec<Choice<FieldValue>>,
        value: FieldValue,
    ) -> Self {
        Self {
            field,
            key: None,
            label,
            choices,
            presentation,
            kind: FieldKind::Remembered,
            step_from: None,
            change_hint: None,
            note: None,
            own_note: None,
            when_unset: value,
            read: |value| FieldValue::Set(value.clone()),
        }
    }

    /// Takes the field out of the `j`/`k` walk.
    pub fn not_in_walk(self) -> Self {
        Self {
            kind: FieldKind::NotInWalk,
            ..self
        }
    }

    /// The choice `h`/`l` step from while the value isn't one of the choices; the first otherwise.
    pub fn step_from(self, value: FieldValue) -> Self {
        Self {
            step_from: Some(value),
            ..self
        }
    }

    /// What the `h`/`l` hint (or an Action's `enter` hint) reads on this field, in place of
    /// "change".
    pub fn change_hint(self, label: String) -> Self {
        Self {
            change_hint: Some(label),
            ..self
        }
    }

    /// The line under the field.
    pub fn note(self, note: String) -> Self {
        Self {
            note: Some(note),
            ..self
        }
    }

    /// The field's own note ([`FieldView::own_note`]), drawn in place of the stock "Set by" note,
    /// which the field then never shows.
    pub fn own_note_replacing_shadowed(
        self,
        note: impl Fn(&FieldValue, Option<&str>) -> Option<String> + 'static,
    ) -> Self {
        Self {
            own_note: Some(Rc::new(note)),
            ..self
        }
    }

    /// The value shown while the device reads none for the key.
    pub fn when_unset(self, value: FieldValue) -> Self {
        Self {
            when_unset: value,
            ..self
        }
    }

    /// How a value the device reads, or a write saves, becomes the field's: a Locale tag that
    /// isn't a choice is **System**, for one.
    pub fn reading(self, read: fn(&ConfigValue) -> FieldValue) -> Self {
        Self { read, ..self }
    }
}

/// What a page passes on to the form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FormInput<F> {
    /// `j`/`k`: the field below or above, or an open list's highlight.
    Move(Step),
    /// `h`/`l`: the focused field's choice after or before.
    Change(Step),
    /// `enter`: opens a dropdown's list or picks its highlight, toggles a checkbox, presses an
    /// action.
    Select,
    /// `space`: toggles a focused checkbox.
    Toggle,
    /// `esc` while a list is open: closes it, changing nothing.
    CloseList,
    /// A click on a field's choice, by its index.
    Choose { field: F, index: usize },
    /// A click on a field itself: opens a dropdown's list, or closes it while open; presses an
    /// action; toggles a checkbox.
    Press(F),
}

/// What the form answers the page, for it to route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FormAnswer<F> {
    /// The field's value was written and is now shown.
    Saved(F, FieldValue),
    /// A remembered field's value changed; nothing was written.
    Changed(F, FieldValue),
    /// An action was pressed.
    Activated(F),
    /// `j`/`k` past the first or last field in the walk: focus stays where it was.
    LeftEnd(Step),
}

/// One field as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldView {
    /// The key the field writes, `None` for a remembered one: the stock shadowed note's selector.
    pub key: Option<ConfigKey>,
    pub label: String,
    /// Each choice's label, in order.
    pub options: Vec<String>,
    /// The choice shown, `None` for a value that isn't one.
    pub chosen: Option<usize>,
    pub focused: bool,
    /// The line under the field.
    pub hint: Option<String>,
    /// The stock note while a source above the file sets the key, unless the field declares its
    /// own.
    pub shadowed: Option<String>,
    /// The field's own note, which it declares in place of the stock one.
    pub own_note: Option<String>,
    pub presentation: Presentation,
    /// A dropdown's list, closed for every other field.
    pub list: SelectList,
}

impl FieldView {
    /// Whether a checkbox is ticked: its choices are unticked then ticked.
    pub fn checked(&self) -> bool {
        self.chosen == Some(1)
    }
}

/// A page's fields as drawn, in the order the page asked for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldViews<F>(Vec<(F, FieldView)>);

impl<F: Copy + Eq> FieldViews<F> {
    /// `field` as drawn.
    pub fn view(&self, field: F) -> Option<&FieldView> {
        self.0
            .iter()
            .find(|(each, _)| *each == field)
            .map(|(_, view)| view)
    }

    /// Whether a dropdown's list is open.
    pub fn list_open(&self) -> bool {
        self.0.iter().any(|(_, view)| view.list.is_open())
    }

    /// The field the keyboard is on, `None` while the page doesn't have it.
    pub fn focused(&self) -> Option<F> {
        self.0
            .iter()
            .find(|(_, view)| view.focused)
            .map(|(field, _)| *field)
    }
}

/// One field and its value now.
struct Field<F> {
    decl: FieldDecl<F>,
    value: FieldValue,
    /// The source above the file that sets the key, if any.
    shadow: Option<String>,
}

/// A This device page's fields: their focus, values, open list and writes.
pub(crate) struct DeviceForm<F> {
    device: Rc<dyn DeviceConfiguration>,
    fields: Vec<Field<F>>,
    focus: Option<F>,
    /// The one open dropdown and its list.
    open: Option<(F, SelectList)>,
}

impl<F: Copy + Eq> DeviceForm<F> {
    /// The form over `fields`, in the walk's order, showing what `snapshot` reads
    /// ([`DeviceSnapshot::shown`]) and writing to `device`.
    pub fn new(
        device: Rc<dyn DeviceConfiguration>,
        snapshot: &DeviceSnapshot,
        fields: Vec<FieldDecl<F>>,
    ) -> Self {
        let fields = fields
            .into_iter()
            .map(|decl| {
                let (value, shadow) = match (decl.kind, decl.key) {
                    (FieldKind::Remembered, _) | (_, None) => (decl.when_unset.clone(), None),
                    (_, Some(key)) => (
                        snapshot
                            .shown(key)
                            .map_or_else(|| decl.when_unset.clone(), decl.read),
                        snapshot.get(key).shadow.clone(),
                    ),
                };
                Field {
                    decl,
                    value,
                    shadow,
                }
            })
            .collect();
        Self {
            device,
            fields,
            focus: None,
            open: None,
        }
    }

    /// The field the keyboard is on, `None` while the page doesn't have it.
    pub fn focused(&self) -> Option<F> {
        self.focus
    }

    /// Puts the keyboard on `field`, as a page does handing focus back to the form.
    pub fn focus(&mut self, field: F) {
        if self.get(field).is_some() {
            self.focus = Some(field);
        }
    }

    /// Whether the page has the keyboard: arriving puts it on the first field in the walk, and
    /// leaving takes it off the fields and closes an open list.
    pub fn set_focused(&mut self, focused: bool) {
        self.focus = if focused { self.walk().next() } else { None };
        self.close_list();
    }

    /// The field's value now.
    pub(crate) fn value(&self, field: F) -> Option<&FieldValue> {
        self.get(field).map(|field| &field.value)
    }

    /// Sets a remembered field's value, as the Client hands it back, answering nothing.
    pub fn set_value(&mut self, field: F, value: FieldValue) {
        if let Some(field) = self.get_mut(field) {
            field.value = value;
        }
    }

    pub fn list_open(&self) -> bool {
        self.open.is_some()
    }

    /// Closes an open list, changing nothing.
    pub fn close_list(&mut self) {
        self.open = None;
    }

    /// Whether the form takes `input` as things stand, so a key it declines goes on to the frame:
    /// a move or change only while a field has the keyboard, and every list key while a list is
    /// open, so neither the frame's `h` nor the Shell's `back` sees one.
    pub fn takes(&self, input: &FormInput<F>) -> bool {
        let open = self.list_open();
        let focused = self.focus.and_then(|field| self.get(field));
        match input {
            FormInput::Move(_) | FormInput::Change(_) => open || focused.is_some(),
            FormInput::Select => {
                open || focused
                    .is_some_and(|field| field.decl.presentation != Presentation::Segmented)
            }
            FormInput::Toggle => {
                open || focused
                    .is_some_and(|field| field.decl.presentation == Presentation::Checkbox)
            }
            FormInput::CloseList => open,
            FormInput::Choose { .. } | FormInput::Press(_) => true,
        }
    }

    /// Takes `input`, writing a change to the device and saying on `notify` how the write went.
    pub fn handle(
        &mut self,
        input: FormInput<F>,
        notify: &mut dyn Notify,
    ) -> Option<FormAnswer<F>> {
        if !self.takes(&input) {
            return None;
        }
        if let Some((open, list)) = self.open {
            match input {
                FormInput::Move(step) => return self.move_highlight(open, list, step),
                // `h`/`l` wait while a list is open, rather than step the value under it.
                FormInput::Change(_) => return None,
                FormInput::Select => return self.pick_highlight(open, list, notify),
                FormInput::Press(field) if field == open => {
                    self.close_list();
                    return None;
                }
                // Another click, a picked row or `esc` closes the open list first.
                _ => self.close_list(),
            }
        }
        match input {
            FormInput::Move(step) => self.move_focus(step),
            FormInput::Change(step) => {
                let field = self.focus?;
                let next = self.stepped(field, step)?;
                self.choose(field, next, notify)
            }
            FormInput::Select => {
                let field = self.focus?;
                self.press(field, notify)
            }
            FormInput::Toggle => {
                let field = self.focus?;
                self.toggle(field, notify)
            }
            // The open list closed above.
            FormInput::CloseList => None,
            FormInput::Choose { field, index } => {
                let value = self.get(field)?.decl.choices.get(index)?.value.clone();
                self.choose(field, value, notify)
            }
            FormInput::Press(field) => {
                let answer = self.press(field, notify);
                // A click on a dropdown puts the keyboard on it while the page has it, so its
                // list keys follow.
                if self.focus.is_some() && self.open.is_some() {
                    self.focus = Some(field);
                }
                answer
            }
        }
    }

    /// Writes `value` to `field` unless it is the value shown already.
    pub fn choose(
        &mut self,
        field: F,
        value: FieldValue,
        notify: &mut dyn Notify,
    ) -> Option<FormAnswer<F>> {
        if self.value(field)? == &value {
            return None;
        }
        self.write(field, value, notify)
    }

    /// Writes `value` to `field` even when it is the value shown, as a Command does. The field
    /// shows it only once saved; a remembered field changes without a write.
    pub fn write(
        &mut self,
        field: F,
        value: FieldValue,
        notify: &mut dyn Notify,
    ) -> Option<FormAnswer<F>> {
        let device = Rc::clone(&self.device);
        let entry = self.get_mut(field)?;
        let key = match (entry.decl.kind, entry.decl.key) {
            (FieldKind::Remembered, _) | (_, None) => {
                entry.value = value.clone();
                return Some(FormAnswer::Changed(field, value));
            }
            (_, Some(key)) => key,
        };
        let saved = match value {
            FieldValue::Set(value) => {
                settle(device.write(value), key, notify).map(|saved| (entry.decl.read)(&saved))
            }
            FieldValue::Unset => {
                settle(device.remove(key), key, notify).map(|()| FieldValue::Unset)
            }
            // A flag has no key to write it under.
            FieldValue::Flag(_) => None,
        }?;
        entry.value = saved.clone();
        Some(FormAnswer::Saved(field, saved))
    }

    /// Writes a value no field shows, as Display's Colour Theme grid does, saying how it went as a
    /// field's write does: the value saved, `None` when it wasn't.
    pub fn write_unfielded(
        &self,
        value: ConfigValue,
        notify: &mut dyn Notify,
    ) -> Option<ConfigValue> {
        let key = value.key();
        settle(self.device.write(value), key, notify)
    }

    /// `field` as drawn.
    pub fn view(&self, field: F) -> Option<FieldView> {
        let entry = self.get(field)?;
        let decl = &entry.decl;
        Some(FieldView {
            key: decl.key,
            label: decl.label.clone(),
            options: decl
                .choices
                .iter()
                .map(|choice| choice.label.clone())
                .collect(),
            chosen: self.chosen(entry),
            focused: self.focus == Some(field),
            hint: decl.note.clone(),
            shadowed: match decl.own_note {
                Some(_) => None,
                None => shadowed_note(entry.shadow.as_deref()),
            },
            own_note: decl
                .own_note
                .as_ref()
                .and_then(|note| note(&entry.value, entry.shadow.as_deref())),
            presentation: decl.presentation,
            list: match self.open {
                Some((open, list)) if open == field => list,
                _ => SelectList::default(),
            },
        })
    }

    /// Each of `fields` as drawn, in their order.
    pub fn views(&self, fields: impl IntoIterator<Item = F>) -> FieldViews<F> {
        FieldViews(
            fields
                .into_iter()
                .filter_map(|field| self.view(field).map(|view| (field, view)))
                .collect(),
        )
    }

    /// The form's keys: an open list's own while one is open; else `j`/`k` and the focused field's
    /// own, read from `keymap` with the Shell's `back` on `back_chord`.
    pub fn hints<E: Clone>(&self, keymap: &Keymap<E>, back_chord: &str) -> Vec<Hint> {
        if self.list_open() {
            return vec![
                Hint::new(keymap.hint_chord(SELECT), msg::settings_hint_pick()),
                Hint::new(
                    keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
                    msg::settings_hint_move(),
                ),
                Hint::new(back_chord.to_string(), msg::settings_hint_close()),
            ];
        }
        match self.focus {
            Some(field) => self.field_hints(field, keymap),
            None => vec![walk_hint(keymap)],
        }
    }

    /// The keys while `field` has the keyboard and no list is open: `j`/`k` and the field's own.
    pub(crate) fn field_hints<E: Clone>(&self, field: F, keymap: &Keymap<E>) -> Vec<Hint> {
        let walk = walk_hint(keymap);
        let Some(decl) = self.get(field).map(|f| &f.decl) else {
            return vec![walk];
        };
        let change = decl
            .change_hint
            .clone()
            .unwrap_or_else(msg::settings_hint_change);
        if decl.presentation == Presentation::Action {
            return vec![walk, Hint::new(keymap.hint_chord(SELECT), change)];
        }
        let mut hints = vec![
            walk,
            Hint::new(keymap.hint_chords(&[PREVIOUS_CHOICE, NEXT_CHOICE]), change),
        ];
        match decl.presentation {
            Presentation::Dropdown => {
                hints.push(Hint::new(
                    keymap.hint_chord(SELECT),
                    msg::settings_hint_open(),
                ));
            }
            Presentation::Checkbox => {
                hints.push(Hint::new(
                    keymap.hint_chord(SELECT),
                    msg::settings_hint_toggle(),
                ));
            }
            Presentation::Segmented | Presentation::Action => {}
        }
        hints
    }

    fn get(&self, field: F) -> Option<&Field<F>> {
        self.fields.iter().find(|entry| entry.decl.field == field)
    }

    fn get_mut(&mut self, field: F) -> Option<&mut Field<F>> {
        self.fields
            .iter_mut()
            .find(|entry| entry.decl.field == field)
    }

    /// The fields `j`/`k` walk, in order.
    fn walk(&self) -> impl Iterator<Item = F> + '_ {
        self.fields
            .iter()
            .filter(|entry| entry.decl.kind != FieldKind::NotInWalk)
            .map(|entry| entry.decl.field)
    }

    fn chosen(&self, entry: &Field<F>) -> Option<usize> {
        position(&entry.decl.choices, &entry.value)
    }

    /// `j`/`k` walk the fields, answering [`FormAnswer::LeftEnd`] past either end.
    fn move_focus(&mut self, step: Step) -> Option<FormAnswer<F>> {
        let at = self
            .focus
            .and_then(|focus| self.walk().position(|field| field == focus));
        let Some(at) = at else {
            let first = self.walk().next();
            self.focus = first;
            return None;
        };
        let next = lib_list_cursor::step(at, self.walk().count(), step);
        if next == at && matches!(step, Step::Next | Step::Prev) {
            return Some(FormAnswer::LeftEnd(step));
        }
        let next = self.walk().nth(next);
        self.focus = next;
        None
    }

    /// The choice `step` from `field`'s, stopping at either end; a value that isn't a choice steps
    /// from the field's `step_from`, else its first. A checkbox's `h` is its first, `l` its last.
    fn stepped(&self, field: F, step: Step) -> Option<FieldValue> {
        let entry = self.get(field)?;
        let choices = &entry.decl.choices;
        match entry.decl.presentation {
            Presentation::Action => None,
            Presentation::Checkbox => match step {
                Step::Prev | Step::First => choices.first(),
                _ => choices.last(),
            }
            .map(|choice| choice.value.clone()),
            Presentation::Segmented | Presentation::Dropdown => {
                let at = self.chosen(entry).unwrap_or_else(|| {
                    entry
                        .decl
                        .step_from
                        .as_ref()
                        .and_then(|from| position(choices, from))
                        .unwrap_or(0)
                });
                let next = lib_list_cursor::step(at, choices.len(), step);
                choices.get(next).map(|choice| choice.value.clone())
            }
        }
    }

    /// `enter` or a click on `field`: opens a dropdown's list, toggles a checkbox, presses an
    /// action.
    fn press(&mut self, field: F, notify: &mut dyn Notify) -> Option<FormAnswer<F>> {
        let entry = self.get(field)?;
        match entry.decl.presentation {
            Presentation::Dropdown => {
                let mut list = SelectList::default();
                list.open(&entry.decl.choices, Some(&entry.value));
                self.open = Some((field, list));
                None
            }
            Presentation::Checkbox => self.toggle(field, notify),
            Presentation::Action => Some(FormAnswer::Activated(field)),
            Presentation::Segmented => None,
        }
    }

    /// Ticks an unticked checkbox, and clears a ticked one.
    fn toggle(&mut self, field: F, notify: &mut dyn Notify) -> Option<FormAnswer<F>> {
        let entry = self.get(field)?;
        if entry.decl.presentation != Presentation::Checkbox {
            return None;
        }
        let choices = &entry.decl.choices;
        let next = if self.chosen(entry) == Some(choices.len().saturating_sub(1)) {
            choices.first()
        } else {
            choices.last()
        }?
        .value
        .clone();
        self.choose(field, next, notify)
    }

    /// `j`/`k` on an open list: its highlight, wrapping at either end.
    fn move_highlight(
        &mut self,
        field: F,
        mut list: SelectList,
        step: Step,
    ) -> Option<FormAnswer<F>> {
        let input = match step {
            Step::Next => DialogInput::SelectNext,
            Step::Prev => DialogInput::SelectPrevious,
            _ => return None,
        };
        let entry = self.get(field)?;
        list.input(input, &entry.decl.choices, Some(&entry.value));
        self.open = Some((field, list));
        None
    }

    /// `enter` on an open list: picks its highlight and closes it.
    fn pick_highlight(
        &mut self,
        field: F,
        mut list: SelectList,
        notify: &mut dyn Notify,
    ) -> Option<FormAnswer<F>> {
        self.close_list();
        let value = list.commit(&self.get(field)?.decl.choices)?;
        self.choose(field, value, notify)
    }
}

/// `value`'s index among `choices`, `None` for a value that isn't one.
fn position(choices: &[Choice<FieldValue>], value: &FieldValue) -> Option<usize> {
    choices.iter().position(|choice| &choice.value == value)
}

/// `j`/`k`'s hint: the field below or above.
fn walk_hint<E: Clone>(keymap: &Keymap<E>) -> Hint {
    Hint::new(
        keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
        msg::settings_hint_field(),
    )
}

/// Says on the status line how the write of `key` went, and hands back the value saved, `None`
/// when it wasn't, so the field keeps what it showed.
fn settle<T>(answer: DeviceAnswer<T>, key: ConfigKey, notify: &mut dyn Notify) -> Option<T> {
    let name = key.name();
    let (status, saved) = match answer {
        DeviceAnswer::Saved { value, file } => {
            (msg::settings_device_saved(name, &file), Some(value))
        }
        DeviceAnswer::Shadowed { source } => (msg::settings_device_shadowed(name, &source), None),
        DeviceAnswer::Refused(DeviceRefusal::NoFile) => (msg::settings_device_no_file(name), None),
        DeviceAnswer::Refused(DeviceRefusal::WriteFailed) => {
            (msg::settings_device_write_failed(name), None)
        }
    };
    notify.notify(Notice::Status(status));
    saved
}

#[cfg(test)]
mod tests {
    use lib_core::{ColourAppearance, RowDensity, StartView};
    use lib_viper::keymap::{Binding, KeyBindingConfig};

    use super::*;
    use crate::fake::{InMemoryDeviceConfiguration, StoredDevice};
    use crate::keymap::TOGGLE;

    /// A toy page's fields, one of every presentation and kind.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Toy {
        Density,
        Appearance,
        Start,
        Lang,
        Hints,
        Folder,
        Sidebar,
    }

    const TOY_BINDINGS: &[Binding<FormInput<Toy>>] = &[
        Binding::hidden(MOVE_DOWN, "j", FormInput::Move(Step::Next)),
        Binding::hidden(MOVE_UP, "k", FormInput::Move(Step::Prev)),
        Binding::hidden(PREVIOUS_CHOICE, "h", FormInput::Change(Step::Prev)),
        Binding::hidden(NEXT_CHOICE, "l", FormInput::Change(Step::Next)),
        Binding::hidden(SELECT, "enter", FormInput::Select),
        Binding::hidden(TOGGLE, "space", FormInput::Toggle),
    ];

    fn choice(value: FieldValue, label: &str) -> Choice<FieldValue> {
        Choice {
            value,
            label: label.to_string(),
        }
    }

    fn set(value: ConfigValue) -> FieldValue {
        FieldValue::Set(value)
    }

    fn locale(tag: &str) -> FieldValue {
        set(ConfigValue::Locale(tag.to_string()))
    }

    fn toy_fields() -> Vec<FieldDecl<Toy>> {
        let density = |density, label| choice(set(ConfigValue::RowDensity(density)), label);
        let start = |start, label| choice(set(ConfigValue::StartView(start)), label);
        let appearance =
            |appearance, label| choice(set(ConfigValue::ColourAppearance(appearance)), label);
        let hints = |on, label| choice(set(ConfigValue::HintBar(on)), label);
        vec![
            FieldDecl::configuration(
                Toy::Density,
                ConfigKey::RowDensity,
                "Density".to_string(),
                Presentation::Segmented,
                vec![
                    density(RowDensity::Compact, "Compact"),
                    density(RowDensity::Regular, "Regular"),
                    density(RowDensity::Roomy, "Roomy"),
                ],
            )
            .when_unset(set(ConfigValue::RowDensity(RowDensity::Regular)))
            .note("Rows".to_string()),
            FieldDecl::configuration(
                Toy::Appearance,
                ConfigKey::ColourAppearance,
                "Appearance".to_string(),
                Presentation::Segmented,
                vec![
                    appearance(ColourAppearance::Light, "Light"),
                    appearance(ColourAppearance::Dark, "Dark"),
                ],
            )
            .not_in_walk(),
            FieldDecl::configuration(
                Toy::Start,
                ConfigKey::StartView,
                "Start".to_string(),
                Presentation::Dropdown,
                vec![
                    start(StartView::Last, "Last"),
                    start(StartView::Dashboard, "Dashboard"),
                    start(StartView::Settings, "Settings"),
                ],
            )
            .when_unset(set(ConfigValue::StartView(StartView::Last)))
            .step_from(set(ConfigValue::StartView(StartView::Dashboard))),
            FieldDecl::configuration(
                Toy::Lang,
                ConfigKey::Locale,
                "Lang".to_string(),
                Presentation::Dropdown,
                vec![
                    choice(FieldValue::Unset, "System"),
                    choice(locale("en-AU"), "en-AU"),
                    choice(locale("en-GB"), "en-GB"),
                ],
            )
            .own_note_replacing_shadowed(|_, source| source.map(|source| format!("Over {source}")))
            .reading(|value| match value {
                ConfigValue::Locale(tag) if tag.eq_ignore_ascii_case("en-gb") => locale("en-GB"),
                ConfigValue::Locale(tag) if tag.eq_ignore_ascii_case("en-au") => locale("en-AU"),
                _ => FieldValue::Unset,
            }),
            FieldDecl::configuration(
                Toy::Hints,
                ConfigKey::HintBar,
                "Hints".to_string(),
                Presentation::Checkbox,
                vec![hints(false, "off"), hints(true, "on")],
            )
            .when_unset(set(ConfigValue::HintBar(true))),
            FieldDecl::configuration(
                Toy::Folder,
                ConfigKey::Data,
                "Folder".to_string(),
                Presentation::Action,
                Vec::new(),
            )
            .change_hint("pick".to_string()),
            FieldDecl::remembered(
                Toy::Sidebar,
                "Sidebar".to_string(),
                Presentation::Checkbox,
                vec![
                    choice(FieldValue::Flag(false), "off"),
                    choice(FieldValue::Flag(true), "on"),
                ],
                FieldValue::Flag(false),
            ),
        ]
    }

    fn form_on(device: Rc<InMemoryDeviceConfiguration>) -> DeviceForm<Toy> {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
        let snapshot = device.read();
        DeviceForm::new(device, &snapshot, toy_fields())
    }

    fn writable() -> (Rc<InMemoryDeviceConfiguration>, DeviceForm<Toy>) {
        let device = Rc::new(InMemoryDeviceConfiguration::writable());
        (device.clone(), form_on(device))
    }

    /// `form` with the keyboard on `field`.
    fn focused_on(form: &mut DeviceForm<Toy>, field: Toy) {
        form.set_focused(true);
        form.focus(field);
    }

    /// Sends each of `inputs`, answering the last answer and the status lines said.
    fn send(
        form: &mut DeviceForm<Toy>,
        inputs: impl IntoIterator<Item = FormInput<Toy>>,
    ) -> (Option<FormAnswer<Toy>>, Vec<String>) {
        let mut notices = Vec::new();
        let mut answer = None;
        for input in inputs {
            answer = form.handle(input, &mut notices);
        }
        let statuses = notices
            .into_iter()
            .filter_map(|notice| match notice {
                Notice::Status(text) => Some(text),
                _ => None,
            })
            .collect();
        (answer, statuses)
    }

    fn chosen(form: &DeviceForm<Toy>, field: Toy) -> Option<usize> {
        form.view(field).and_then(|view| view.chosen)
    }

    fn highlight(form: &DeviceForm<Toy>, field: Toy) -> Option<usize> {
        form.view(field).and_then(|view| view.list.highlight())
    }

    fn pairs(form: &DeviceForm<Toy>) -> Vec<(String, String)> {
        form.hints(
            &Keymap::new(TOY_BINDINGS, &KeyBindingConfig::default()),
            "esc",
        )
        .into_iter()
        .map(|hint| (hint.chord, hint.action))
        .collect()
    }

    #[test]
    fn focus_arrives_on_the_first_field_and_leaves_the_fields() {
        let (_, mut form) = writable();
        assert_eq!(form.focused(), None);
        assert!(!form.takes(&FormInput::Move(Step::Next)));

        form.set_focused(true);
        assert_eq!(form.focused(), Some(Toy::Density));
        assert!(form.view(Toy::Density).is_some_and(|view| view.focused));
        form.set_focused(false);
        assert_eq!(form.focused(), None);
    }

    #[test]
    fn j_and_k_walk_the_fields_skip_one_not_in_the_walk_and_answer_the_ends() {
        let (_, mut form) = writable();
        form.set_focused(true);

        send(&mut form, [FormInput::Move(Step::Next)]);
        assert_eq!(form.focused(), Some(Toy::Start), "Appearance is skipped");
        send(
            &mut form,
            std::iter::repeat_n(FormInput::Move(Step::Next), 4),
        );
        assert_eq!(form.focused(), Some(Toy::Sidebar));

        let (answer, _) = send(&mut form, [FormInput::Move(Step::Next)]);
        assert_eq!(answer, Some(FormAnswer::LeftEnd(Step::Next)));
        assert_eq!(form.focused(), Some(Toy::Sidebar), "focus stays");

        send(
            &mut form,
            std::iter::repeat_n(FormInput::Move(Step::Prev), 5),
        );
        assert_eq!(form.focused(), Some(Toy::Density));
        let (answer, _) = send(&mut form, [FormInput::Move(Step::Prev)]);
        assert_eq!(answer, Some(FormAnswer::LeftEnd(Step::Prev)));
    }

    #[test]
    fn a_field_not_in_the_walk_is_written_by_a_click() {
        let (device, mut form) = writable();

        let (answer, statuses) = send(
            &mut form,
            [FormInput::Choose {
                field: Toy::Appearance,
                index: 1,
            }],
        );

        let dark = set(ConfigValue::ColourAppearance(ColourAppearance::Dark));
        assert_eq!(answer, Some(FormAnswer::Saved(Toy::Appearance, dark)));
        assert_eq!(
            device.stored().get(ConfigKey::ColourAppearance),
            Some(&ConfigValue::ColourAppearance(ColourAppearance::Dark))
        );
        assert_eq!(
            statuses,
            ["colour_appearance saved to personal-ledger.conf"]
        );
    }

    #[test]
    fn h_and_l_step_a_segmented_field_saving_each_and_stopping_at_the_ends() {
        let (device, mut form) = writable();
        form.set_focused(true);
        assert_eq!(chosen(&form, Toy::Density), Some(1), "the default");

        let (answer, statuses) = send(&mut form, [FormInput::Change(Step::Next)]);

        let roomy = set(ConfigValue::RowDensity(RowDensity::Roomy));
        assert_eq!(answer, Some(FormAnswer::Saved(Toy::Density, roomy)));
        assert_eq!(statuses, ["row_density saved to personal-ledger.conf"]);
        assert_eq!(chosen(&form, Toy::Density), Some(2));
        let (answer, statuses) = send(&mut form, [FormInput::Change(Step::Next)]);
        assert_eq!(
            (answer, statuses),
            (None, vec![]),
            "the last stays, unwritten"
        );

        send(
            &mut form,
            std::iter::repeat_n(FormInput::Change(Step::Prev), 3),
        );
        assert_eq!(
            device.stored().get(ConfigKey::RowDensity),
            Some(&ConfigValue::RowDensity(RowDensity::Compact))
        );
    }

    #[test]
    fn a_value_that_isnt_a_choice_shows_none_chosen_and_steps_from_step_from() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::writable()
                .reading([ConfigValue::StartView(StartView::Noun("units".into()))]),
        );
        let mut form = form_on(device.clone());
        assert_eq!(chosen(&form, Toy::Start), None);
        focused_on(&mut form, Toy::Start);

        send(&mut form, [FormInput::Change(Step::Next)]);

        assert_eq!(
            device.stored().get(ConfigKey::StartView),
            Some(&ConfigValue::StartView(StartView::Settings)),
            "the step after the Dashboard"
        );
    }

    #[test]
    fn enter_opens_a_dropdown_on_its_choice_and_j_k_and_enter_pick_from_it() {
        let (device, mut form) = writable();
        focused_on(&mut form, Toy::Start);
        assert!(form.takes(&FormInput::Select));

        send(&mut form, [FormInput::Select]);
        assert_eq!(
            highlight(&form, Toy::Start),
            Some(0),
            "on Last, the default"
        );
        assert_eq!(
            device.stored(),
            StoredDevice::default(),
            "opening writes nothing"
        );

        send(
            &mut form,
            [
                FormInput::Move(Step::Next),
                FormInput::Move(Step::Next),
                FormInput::Move(Step::Prev),
            ],
        );
        assert_eq!(form.focused(), Some(Toy::Start), "j/k move the highlight");
        assert_eq!(highlight(&form, Toy::Start), Some(1));

        let (answer, statuses) = send(&mut form, [FormInput::Select]);

        assert!(!form.list_open());
        let dashboard = set(ConfigValue::StartView(StartView::Dashboard));
        assert_eq!(answer, Some(FormAnswer::Saved(Toy::Start, dashboard)));
        assert_eq!(statuses, ["start_view saved to personal-ledger.conf"]);
    }

    #[test]
    fn an_open_lists_highlight_wraps_at_either_end() {
        let (_, mut form) = writable();
        focused_on(&mut form, Toy::Start);
        send(&mut form, [FormInput::Select, FormInput::Move(Step::Prev)]);
        assert_eq!(highlight(&form, Toy::Start), Some(2));
    }

    #[test]
    fn esc_closes_the_open_list_changing_nothing_and_is_declined_once_closed() {
        let (device, mut form) = writable();
        focused_on(&mut form, Toy::Start);
        assert!(!form.takes(&FormInput::CloseList), "nothing open");
        send(&mut form, [FormInput::Select, FormInput::Move(Step::Next)]);
        assert!(form.takes(&FormInput::CloseList));

        let (answer, statuses) = send(&mut form, [FormInput::CloseList]);

        assert_eq!((answer, statuses), (None, vec![]));
        assert!(!form.list_open());
        assert_eq!(device.stored(), StoredDevice::default());
        assert_eq!(form.focused(), Some(Toy::Start));
    }

    #[test]
    fn h_and_l_wait_while_a_list_is_open_and_step_it_while_closed() {
        let (device, mut form) = writable();
        focused_on(&mut form, Toy::Start);
        send(
            &mut form,
            [FormInput::Select, FormInput::Change(Step::Next)],
        );
        assert!(form.takes(&FormInput::Change(Step::Prev)), "swallowed");
        assert!(form.list_open());
        assert_eq!(device.stored(), StoredDevice::default());

        send(
            &mut form,
            [FormInput::CloseList, FormInput::Change(Step::Next)],
        );
        assert_eq!(
            device.stored().get(ConfigKey::StartView),
            Some(&ConfigValue::StartView(StartView::Dashboard))
        );
    }

    #[test]
    fn leaving_the_page_closes_an_open_list() {
        let (_, mut form) = writable();
        focused_on(&mut form, Toy::Start);
        send(&mut form, [FormInput::Select]);
        form.set_focused(false);
        assert!(!form.list_open());
    }

    #[test]
    fn a_click_opens_a_list_closes_the_other_and_a_second_click_closes_it() {
        let (device, mut form) = writable();
        send(&mut form, [FormInput::Press(Toy::Start)]);
        assert_eq!(highlight(&form, Toy::Start), Some(0));
        assert_eq!(form.focused(), None, "the index keeps the keyboard");

        send(&mut form, [FormInput::Press(Toy::Lang)]);
        assert_eq!(highlight(&form, Toy::Start), None, "one list at a time");
        assert_eq!(highlight(&form, Toy::Lang), Some(0));

        send(&mut form, [FormInput::Press(Toy::Lang)]);
        assert!(!form.list_open());
        assert_eq!(device.stored(), StoredDevice::default());
    }

    #[test]
    fn a_click_on_a_dropdown_puts_the_keyboard_on_it_while_the_page_has_it() {
        let (_, mut form) = writable();
        form.set_focused(true);
        send(&mut form, [FormInput::Press(Toy::Lang)]);
        assert_eq!(form.focused(), Some(Toy::Lang));
    }

    #[test]
    fn a_click_on_a_row_picks_it_and_closes_the_list() {
        let (device, mut form) = writable();
        send(
            &mut form,
            [
                FormInput::Press(Toy::Start),
                FormInput::Choose {
                    field: Toy::Start,
                    index: 2,
                },
            ],
        );
        assert!(!form.list_open());
        assert_eq!(
            device.stored().get(ConfigKey::StartView),
            Some(&ConfigValue::StartView(StartView::Settings))
        );
    }

    #[test]
    fn a_checkboxs_h_clears_l_ticks_and_enter_and_space_toggle() {
        let (device, mut form) = writable();
        focused_on(&mut form, Toy::Hints);
        let hint_bar =
            |device: &InMemoryDeviceConfiguration| device.stored().get(ConfigKey::HintBar).cloned();

        send(&mut form, [FormInput::Change(Step::Next)]);
        assert_eq!(hint_bar(&device), None, "ticked already, so unwritten");
        send(&mut form, [FormInput::Change(Step::Prev)]);
        assert_eq!(hint_bar(&device), Some(ConfigValue::HintBar(false)));
        send(&mut form, [FormInput::Toggle]);
        assert_eq!(hint_bar(&device), Some(ConfigValue::HintBar(true)));
        send(&mut form, [FormInput::Select]);
        assert_eq!(hint_bar(&device), Some(ConfigValue::HintBar(false)));
        assert!(!form.list_open(), "enter on a checkbox opens nothing");
    }

    #[test]
    fn enter_on_an_action_answers_activated_and_writes_nothing() {
        let (device, mut form) = writable();
        focused_on(&mut form, Toy::Folder);

        assert_eq!(
            send(&mut form, [FormInput::Select]).0,
            Some(FormAnswer::Activated(Toy::Folder))
        );
        assert_eq!(
            send(&mut form, [FormInput::Press(Toy::Folder)]).0,
            Some(FormAnswer::Activated(Toy::Folder))
        );
        assert_eq!(
            send(&mut form, [FormInput::Change(Step::Next)]).0,
            None,
            "no choices to step"
        );
        assert_eq!(device.stored(), StoredDevice::default());
    }

    #[test]
    fn a_remembered_field_answers_its_change_and_writes_nothing() {
        let (device, mut form) = writable();
        focused_on(&mut form, Toy::Sidebar);

        let (answer, statuses) = send(&mut form, [FormInput::Toggle]);

        assert_eq!(
            answer,
            Some(FormAnswer::Changed(Toy::Sidebar, FieldValue::Flag(true)))
        );
        assert!(statuses.is_empty());
        assert_eq!(device.stored(), StoredDevice::default());
        assert_eq!(chosen(&form, Toy::Sidebar), Some(1));

        form.set_value(Toy::Sidebar, FieldValue::Flag(false));
        assert_eq!(
            chosen(&form, Toy::Sidebar),
            Some(0),
            "as the Client hands it back"
        );
    }

    #[test]
    fn a_shadowed_field_is_noted_and_a_refused_write_keeps_what_was_shown() {
        let device = Rc::new(InMemoryDeviceConfiguration::shadowing(
            ConfigKey::RowDensity,
            "--roomy",
        ));
        let mut form = form_on(device.clone());
        assert_eq!(
            form.view(Toy::Density).and_then(|view| view.shadowed),
            Some("Set by --roomy".to_string())
        );
        assert_eq!(form.view(Toy::Start).and_then(|view| view.shadowed), None);

        let (answer, statuses) = send(
            &mut form,
            [FormInput::Choose {
                field: Toy::Density,
                index: 0,
            }],
        );

        assert_eq!(answer, None);
        assert_eq!(chosen(&form, Toy::Density), Some(1));
        assert_eq!(device.stored(), StoredDevice::default());
        assert!(statuses[0].contains("--roomy"));

        let mut form = form_on(Rc::new(InMemoryDeviceConfiguration {
            fail_writes: true,
            ..InMemoryDeviceConfiguration::writable()
        }));
        let (answer, statuses) = send(
            &mut form,
            [FormInput::Choose {
                field: Toy::Density,
                index: 0,
            }],
        );
        assert_eq!(answer, None);
        assert_eq!(chosen(&form, Toy::Density), Some(1));
        assert_eq!(
            statuses,
            ["Couldn't save row_density to personal-ledger.conf: the log says why"]
        );
    }

    #[test]
    fn a_key_written_while_shadowed_shows_the_files_value_and_is_written() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::shadowing(ConfigKey::Locale, "--locale en-AU")
                .reading([ConfigValue::Locale("en-AU".to_string())])
                .storing([ConfigValue::Locale("en-gb".to_string())]),
        );
        let mut form = form_on(device.clone());
        let view = form.view(Toy::Lang);
        assert_eq!(
            view.as_ref().and_then(|view| view.chosen),
            Some(2),
            "the file's"
        );
        assert_eq!(
            view.as_ref().and_then(|view| view.shadowed.clone()),
            None,
            "the field draws its own note"
        );
        assert_eq!(
            view.and_then(|view| view.own_note),
            Some("Over --locale en-AU".to_string())
        );

        let (answer, statuses) = send(
            &mut form,
            [FormInput::Choose {
                field: Toy::Lang,
                index: 1,
            }],
        );

        assert_eq!(answer, Some(FormAnswer::Saved(Toy::Lang, locale("en-AU"))));
        assert_eq!(statuses, ["locale saved to personal-ledger.conf"]);
        assert_eq!(chosen(&form, Toy::Lang), Some(1));
    }

    #[test]
    fn unset_removes_the_key() {
        let device = Rc::new(
            InMemoryDeviceConfiguration::writable()
                .storing([ConfigValue::Locale("en-GB".to_string())]),
        );
        let mut form = form_on(device.clone());

        let (answer, _) = send(
            &mut form,
            [FormInput::Choose {
                field: Toy::Lang,
                index: 0,
            }],
        );

        assert_eq!(
            answer,
            Some(FormAnswer::Saved(Toy::Lang, FieldValue::Unset))
        );
        assert_eq!(device.stored(), StoredDevice::default());
        assert_eq!(chosen(&form, Toy::Lang), Some(0));
    }

    #[test]
    fn choose_skips_the_value_shown_and_write_always_writes() {
        let (device, mut form) = writable();
        let regular = set(ConfigValue::RowDensity(RowDensity::Regular));
        let mut notices = Vec::new();

        assert_eq!(
            form.choose(Toy::Density, regular.clone(), &mut notices),
            None
        );
        assert!(notices.is_empty());

        assert_eq!(
            form.write(Toy::Density, regular.clone(), &mut notices),
            Some(FormAnswer::Saved(Toy::Density, regular))
        );
        assert_eq!(
            device.stored().get(ConfigKey::RowDensity),
            Some(&ConfigValue::RowDensity(RowDensity::Regular))
        );
    }

    #[test]
    fn a_write_no_field_shows_settles_as_a_fields_does() {
        let (device, form) = writable();
        let mut notices = Vec::new();

        let saved =
            form.write_unfielded(ConfigValue::ColourTheme("nord".to_string()), &mut notices);

        assert_eq!(saved, Some(ConfigValue::ColourTheme("nord".to_string())));
        assert_eq!(
            device.stored().get(ConfigKey::ColourTheme),
            Some(&ConfigValue::ColourTheme("nord".to_string()))
        );
        assert_eq!(
            notices,
            [Notice::Status(
                "colour_theme saved to personal-ledger.conf".to_string()
            )]
        );

        let form = form_on(Rc::new(InMemoryDeviceConfiguration::default()));
        let mut notices = Vec::new();
        assert_eq!(
            form.write_unfielded(ConfigValue::ColourTheme("nord".to_string()), &mut notices),
            None
        );
        assert_eq!(notices.len(), 1);
    }

    #[test]
    fn takes_declines_what_the_frame_should_have() {
        let (_, mut form) = writable();
        assert!(
            !form.takes(&FormInput::Change(Step::Next)),
            "no field focused"
        );
        assert!(form.takes(&FormInput::Press(Toy::Start)), "a click always");
        form.set_focused(true);
        assert!(
            !form.takes(&FormInput::Select),
            "enter is the frame's on a segment"
        );
        assert!(
            !form.takes(&FormInput::Toggle),
            "space is the frame's off a checkbox"
        );
        form.focus(Toy::Folder);
        assert!(form.takes(&FormInput::Select));
        form.focus(Toy::Hints);
        assert!(form.takes(&FormInput::Toggle));
    }

    #[test]
    fn the_hints_follow_the_focused_field_and_an_open_list() {
        let (_, mut form) = writable();
        form.set_focused(true);
        let pair = |chord: &str, action: &str| (chord.to_string(), action.to_string());
        assert_eq!(pairs(&form), [pair("j/k", "field"), pair("h/l", "change")]);

        form.focus(Toy::Start);
        assert_eq!(
            pairs(&form),
            [
                pair("j/k", "field"),
                pair("h/l", "change"),
                pair("enter", "open")
            ]
        );
        send(&mut form, [FormInput::Select]);
        assert_eq!(
            pairs(&form),
            [
                pair("enter", "pick"),
                pair("j/k", "move"),
                pair("esc", "close")
            ]
        );
        send(&mut form, [FormInput::CloseList]);

        form.focus(Toy::Hints);
        assert_eq!(
            pairs(&form),
            [
                pair("j/k", "field"),
                pair("h/l", "change"),
                pair("enter", "toggle")
            ]
        );
        form.focus(Toy::Folder);
        assert_eq!(pairs(&form), [pair("j/k", "field"), pair("enter", "pick")]);
    }

    #[test]
    fn a_field_view_carries_its_label_options_and_note() {
        let (_, form) = writable();
        let view = form.view(Toy::Density);
        assert_eq!(
            view,
            Some(FieldView {
                key: Some(ConfigKey::RowDensity),
                label: "Density".to_string(),
                options: vec![
                    "Compact".to_string(),
                    "Regular".to_string(),
                    "Roomy".to_string()
                ],
                chosen: Some(1),
                focused: false,
                hint: Some("Rows".to_string()),
                shadowed: None,
                own_note: None,
                presentation: Presentation::Segmented,
                list: SelectList::default(),
            })
        );
    }
}
