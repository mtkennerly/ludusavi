use std::ops::RangeInclusive;

use iced::{widget as w, Alignment, Length};

use crate::{
    gui::{
        common::{Message, Operation, UndoSubject},
        icon::Icon,
        style::{self, Theme},
    },
    lang::TRANSLATOR,
};

pub type Renderer = iced::Renderer;

pub type Element<'a> = iced::Element<'a, Message, Theme, Renderer>;

pub type Button<'a> = w::Button<'a, Message, Theme, Renderer>;
pub type Checkbox<'a> = w::Checkbox<'a, Message, Theme, Renderer>;
pub type Column<'a> = w::Column<'a, Message, Theme, Renderer>;
pub type Container<'a> = w::Container<'a, Message, Theme, Renderer>;
pub type PickList<'a, T, L, V> = w::PickList<'a, T, L, V, Message, Theme, Renderer>;
pub type ProgressBar<'a> = w::ProgressBar<'a, Theme>;
pub type Row<'a> = w::Row<'a, Message, Theme, Renderer>;
pub type Scrollable<'a> = w::Scrollable<'a, Message, Theme, Renderer>;
pub type Stack<'a> = w::Stack<'a, Message, Theme, Renderer>;
pub type Text<'a> = w::Text<'a, Theme, Renderer>;
pub type TextInput<'a> = w::TextInput<'a, Message, Theme, Renderer>;
pub type Tooltip<'a> = w::Tooltip<'a, Message, Theme, Renderer>;
pub type Undoable<'a, F> = crate::gui::undoable::Undoable<'a, Message, Theme, Renderer, F>;

pub use w::Space;

pub fn checkbox<'a>(
    label: impl w::text::IntoFragment<'a>,
    is_checked: bool,
    f: impl Fn(bool) -> Message + 'a,
) -> Checkbox<'a> {
    Checkbox::new(is_checked)
        .label(label)
        .on_toggle(f)
        .size(20)
        .text_shaping(w::text::Shaping::Advanced)
}

pub fn pick_list<'a, T, L, V>(
    options: L,
    selected: Option<V>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> PickList<'a, T, L, V>
where
    T: ToString + PartialEq + Clone,
    L: std::borrow::Borrow<[T]> + 'a,
    V: std::borrow::Borrow<T> + 'a,
    Message: Clone,
    Renderer: iced::advanced::text::Renderer,
{
    PickList::new(options, selected, on_selected)
        .text_shaping(w::text::Shaping::Advanced)
        .padding(5)
}

pub fn text<'a>(content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    Text::new(content).shaping(w::text::Shaping::Advanced)
}

pub mod id {
    use iced::widget::Id;
    use std::sync::LazyLock;

    pub static BACKUP_SCROLL: LazyLock<Id> = LazyLock::new(Id::unique);
    pub static RESTORE_SCROLL: LazyLock<Id> = LazyLock::new(Id::unique);
    pub static CUSTOM_GAMES_SCROLL: LazyLock<Id> = LazyLock::new(Id::unique);
    pub static OTHER_SCROLL: LazyLock<Id> = LazyLock::new(Id::unique);
    pub static MODAL_SCROLL: LazyLock<Id> = LazyLock::new(Id::unique);

    pub static BACKUP_SEARCH: LazyLock<Id> = LazyLock::new(Id::unique);
    pub static RESTORE_SEARCH: LazyLock<Id> = LazyLock::new(Id::unique);
    pub static CUSTOM_GAMES_SEARCH: LazyLock<Id> = LazyLock::new(Id::unique);

    pub fn backup_scroll() -> Id {
        (*BACKUP_SCROLL).clone()
    }

    pub fn restore_scroll() -> Id {
        (*RESTORE_SCROLL).clone()
    }

    pub fn custom_games_scroll() -> Id {
        (*CUSTOM_GAMES_SCROLL).clone()
    }

    pub fn other_scroll() -> Id {
        (*OTHER_SCROLL).clone()
    }

    pub fn modal_scroll() -> Id {
        (*MODAL_SCROLL).clone()
    }

    pub fn backup_search() -> Id {
        (*BACKUP_SEARCH).clone()
    }

    pub fn restore_search() -> Id {
        (*RESTORE_SEARCH).clone()
    }

    pub fn custom_games_search() -> Id {
        (*CUSTOM_GAMES_SEARCH).clone()
    }
}

pub fn number_input<'a>(
    value: i32,
    label: String,
    range: RangeInclusive<i32>,
    change: impl Fn(i32) -> Message,
) -> Element<'a> {
    Container::new(
        Row::new()
            .spacing(5)
            .align_y(Alignment::Center)
            .push(text(label))
            .push(text(value.to_string()))
            .push({
                Button::new(Icon::Remove.text().width(Length::Shrink))
                    .on_press_if(&value > range.start(), || (change)(value - 1))
                    .class(style::Button::Negative)
                    .padding(5)
            })
            .push({
                Button::new(Icon::Add.text().width(Length::Shrink))
                    .on_press_if(&value < range.end(), || (change)(value + 1))
                    .class(style::Button::Primary)
                    .padding(5)
            }),
    )
    .into()
}

pub fn text_editor<'a>(
    content: &'a w::text_editor::Content,
    on_action: impl Fn(w::text_editor::Action) -> Message + 'a,
    undo_subject: UndoSubject,
) -> Element<'a> {
    w::text_editor(content)
        .on_action(on_action)
        .key_binding(move |event| {
            use crate::gui::undoable;
            use iced::keyboard::Key;
            use w::text_editor::{Binding, Status};

            match event.status {
                Status::Active | Status::Hovered | Status::Disabled => None,
                Status::Focused { .. } => match event.key.as_ref() {
                    Key::Character("z") if event.modifiers.command() && event.modifiers.shift() => Some(
                        Binding::Custom(Message::UndoRedo(undoable::Action::Redo, undo_subject.clone())),
                    ),
                    Key::Character("z") if event.modifiers.command() => Some(Binding::Custom(Message::UndoRedo(
                        undoable::Action::Undo,
                        undo_subject.clone(),
                    ))),
                    Key::Character("y") if event.modifiers.command() => Some(Binding::Custom(Message::UndoRedo(
                        undoable::Action::Redo,
                        undo_subject.clone(),
                    ))),
                    _ => Binding::from_key_press(event),
                },
            }
        })
        .into()
}

#[derive(Default)]
pub struct Progress {
    pub max: f32,
    pub current: f32,
    prepared: bool,
    start_time: Option<chrono::DateTime<chrono::Utc>>,
    current_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Progress {
    pub fn visible(&self) -> bool {
        self.max > 0.0
    }

    pub fn reset(&mut self) {
        self.max = 0.0;
        self.current = 0.0;
        self.prepared = false;
        self.start_time = None;
        self.current_time = None;
    }

    pub fn start(&mut self) {
        self.max = 100.0;
        self.current = 0.0;
        self.prepared = false;
        self.start_time = Some(chrono::Utc::now());
    }

    pub fn step(&mut self) {
        self.current += 1.0;
    }

    pub fn set(&mut self, current: f32, max: f32) {
        self.current = current;
        self.max = max;
        self.prepared = true;
    }

    pub fn set_max(&mut self, max: f32) {
        self.max = max;
        self.prepared = true;
    }

    pub fn update_time(&mut self) {
        self.current_time = Some(chrono::Utc::now());
    }

    fn game_count(&self) -> String {
        format!("{}: {} / {}", TRANSLATOR.total_games(), self.current, self.max)
    }

    fn cloud_count(&self) -> String {
        TRANSLATOR.cloud_progress(self.current as u64, self.max as u64)
    }

    pub fn view(&self, operation: &Operation) -> Element {
        let label = match operation {
            Operation::Idle => None,
            Operation::Backup {
                checking_cloud,
                syncing_cloud,
                ..
            } => Some(if *checking_cloud || *syncing_cloud {
                TRANSLATOR.cloud_label()
            } else {
                TRANSLATOR.scan_label()
            }),
            Operation::Restore { checking_cloud, .. } => Some(if *checking_cloud {
                TRANSLATOR.cloud_label()
            } else {
                TRANSLATOR.scan_label()
            }),
            Operation::ValidateBackups { .. } => Some(TRANSLATOR.validate_button()),
            Operation::Cloud { .. } => Some(TRANSLATOR.cloud_label()),
        };

        let elapsed = self.start_time.as_ref().map(|start| {
            let current = self.current_time.as_ref().unwrap_or(start);
            let elapsed = current.time() - start.time();
            format!(
                "({:0>2}:{:0>2}:{:0>2})",
                elapsed.num_hours(),
                elapsed.num_minutes() % 60,
                elapsed.num_seconds() % 60,
            )
        });

        let count = if !self.prepared {
            None
        } else {
            match operation {
                Operation::Idle => None,
                Operation::Backup {
                    checking_cloud,
                    syncing_cloud,
                    ..
                } => Some(if *checking_cloud || *syncing_cloud {
                    self.cloud_count()
                } else {
                    self.game_count()
                }),
                Operation::Restore { checking_cloud, .. } => Some(if *checking_cloud {
                    self.cloud_count()
                } else {
                    self.game_count()
                }),
                Operation::ValidateBackups { .. } => Some(self.game_count()),
                Operation::Cloud { .. } => Some(self.cloud_count()),
            }
        };

        let text_size = 12;

        Container::new(
            Button::new(
                Row::new()
                    .width(Length::Fill)
                    .spacing(5)
                    .padding([0, 5])
                    .align_y(Alignment::Center)
                    .push(label.map(|x| text(x).size(text_size)))
                    .push(elapsed.map(|x| text(x).size(text_size)))
                    .push(ProgressBar::new(0.0..=self.max, self.current).girth(8))
                    .push(count.map(|x| text(x).size(text_size))),
            )
            .on_press_maybe(match operation {
                Operation::Idle | Operation::Cloud { .. } => None,
                Operation::Backup { .. } | Operation::Restore { .. } | Operation::ValidateBackups { .. } => {
                    Some(Message::ShowScanActiveGames)
                }
            })
            .padding(0)
            .class(style::Button::Bare),
        )
        .height(16)
        .class(style::Container::ModalBackground)
        .into()
    }
}

pub trait IcedParentExt<'a> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>;
}

impl<'a> IcedParentExt<'a> for Column<'a> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>,
    {
        if condition {
            self.push(element().into())
        } else {
            self
        }
    }
}

impl<'a> IcedParentExt<'a> for Row<'a> {
    fn push_if<E>(self, condition: bool, element: impl FnOnce() -> E) -> Self
    where
        E: Into<Element<'a>>,
    {
        if condition {
            self.push(element().into())
        } else {
            self
        }
    }
}

pub trait IcedButtonExt<'a> {
    fn on_press_if(self, condition: bool, msg: impl FnOnce() -> Message) -> Self;
}

impl<'a> IcedButtonExt<'a> for Button<'a> {
    fn on_press_if(self, condition: bool, msg: impl FnOnce() -> Message) -> Self {
        if condition {
            self.on_press(msg())
        } else {
            self
        }
    }
}

pub mod operation {
    use iced::{
        advanced::widget::{
            operate,
            operation::{Outcome, Scrollable},
            Operation,
        },
        widget::{scrollable::AbsoluteOffset, Id},
        Rectangle, Task, Vector,
    };

    pub fn container_scroll_offset(id: Id) -> Task<Option<AbsoluteOffset>> {
        struct ContainerScrollOffset {
            target: Id,
            offset: Option<AbsoluteOffset>,
            anchor: Option<f32>,
        }

        impl Operation<Option<AbsoluteOffset>> for ContainerScrollOffset {
            fn scrollable(
                &mut self,
                _id: Option<&Id>,
                bounds: Rectangle,
                _content_bounds: Rectangle,
                _translation: Vector,
                _state: &mut dyn Scrollable,
            ) {
                self.anchor = Some(bounds.y);
            }

            fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
                if self.offset.is_some() {
                    return;
                }

                if id == Some(&self.target) {
                    self.offset = Some(AbsoluteOffset {
                        x: 0.0,
                        y: bounds.y - self.anchor.unwrap_or(0.0),
                    });
                }
            }

            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<AbsoluteOffset>>)) {
                operate(self)
            }

            fn finish(&self) -> Outcome<Option<AbsoluteOffset>> {
                Outcome::Some(self.offset)
            }
        }

        operate(ContainerScrollOffset {
            target: id,
            offset: None,
            anchor: None,
        })
    }
}
