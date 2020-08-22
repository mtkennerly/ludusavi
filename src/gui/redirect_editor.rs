use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, EditAction, RedirectEditActionField},
        common::{Message, OngoingOperation},
        icon::Icon,
        style,
    },
    lang::Translator,
    shortcuts::TextHistory,
};

use iced::{button, scrollable, text_input, Button, Container, Length, Row, Scrollable, Space, TextInput};

#[derive(Default)]
pub struct RedirectEditorRow {
    button_state: button::State,
    pub source_text_state: text_input::State,
    pub source_text_history: TextHistory,
    source_browse_button_state: button::State,
    pub target_text_state: text_input::State,
    pub target_text_history: TextHistory,
    target_browse_button_state: button::State,
}

impl RedirectEditorRow {
    pub fn new(initial_source: &str, initial_target: &str) -> Self {
        Self {
            source_text_history: TextHistory::new(initial_source, 100),
            target_text_history: TextHistory::new(initial_target, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct RedirectEditor {
    scroll: scrollable::State,
    pub rows: Vec<RedirectEditorRow>,
}

impl RedirectEditor {
    pub fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        let redirects = config.get_redirects();
        if redirects.is_empty() {
            Container::new(Space::new(Length::Units(0), Length::Units(0)))
        } else {
            Container::new({
                self.rows.iter_mut().enumerate().fold(
                    Scrollable::new(&mut self.scroll)
                        .width(Length::Fill)
                        .max_height(100)
                        .style(style::Scrollable),
                    |parent: Scrollable<'_, Message>, (i, x)| {
                        parent
                            .push(
                                Row::new()
                                    .spacing(20)
                                    .push(Space::new(Length::Units(0), Length::Units(0)))
                                    .push(
                                        Button::new(&mut x.button_state, Icon::RemoveCircle.as_text())
                                            .on_press(Message::EditedRedirect(EditAction::Remove(i), None))
                                            .style(style::Button::Negative),
                                    )
                                    .push(
                                        TextInput::new(
                                            &mut x.source_text_state,
                                            &translator.redirect_source_placeholder(),
                                            &redirects[i].source.raw(),
                                            move |v| {
                                                Message::EditedRedirect(
                                                    EditAction::Change(i, v),
                                                    Some(RedirectEditActionField::Source),
                                                )
                                            },
                                        )
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push(
                                        Button::new(&mut x.source_browse_button_state, Icon::FolderOpen.as_text())
                                            .on_press(match operation {
                                                None => Message::BrowseDir(BrowseSubject::RedirectSource(i)),
                                                Some(_) => Message::Ignore,
                                            })
                                            .style(match operation {
                                                None => style::Button::Primary,
                                                Some(_) => style::Button::Disabled,
                                            }),
                                    )
                                    .push(
                                        TextInput::new(
                                            &mut x.target_text_state,
                                            &translator.redirect_target_placeholder(),
                                            &redirects[i].target.raw(),
                                            move |v| {
                                                Message::EditedRedirect(
                                                    EditAction::Change(i, v),
                                                    Some(RedirectEditActionField::Target),
                                                )
                                            },
                                        )
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push(
                                        Button::new(&mut x.target_browse_button_state, Icon::FolderOpen.as_text())
                                            .on_press(match operation {
                                                None => Message::BrowseDir(BrowseSubject::RedirectTarget(i)),
                                                Some(_) => Message::Ignore,
                                            })
                                            .style(match operation {
                                                None => style::Button::Primary,
                                                Some(_) => style::Button::Disabled,
                                            }),
                                    )
                                    .push(Space::new(Length::Units(0), Length::Units(0))),
                            )
                            .push(Row::new().push(Space::new(Length::Units(0), Length::Units(5))))
                    },
                )
            })
        }
    }
}
