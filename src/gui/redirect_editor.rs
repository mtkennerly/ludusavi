use crate::{
    config::{Config, RedirectKind},
    gui::{
        common::{BrowseSubject, EditAction, Message, RedirectEditActionField},
        icon::Icon,
        style,
    },
    lang::Translator,
    shortcuts::TextHistory,
};

use iced::{button, pick_list, text_input, Button, Column, Container, Length, PickList, Row, TextInput};

#[derive(Default)]
pub struct RedirectEditorRow {
    remove_button_state: button::State,
    pub source_text_state: text_input::State,
    pub source_text_history: TextHistory,
    source_browse_button_state: button::State,
    pub target_text_state: text_input::State,
    pub target_text_history: TextHistory,
    target_browse_button_state: button::State,
    pick_list: pick_list::State<RedirectKind>,
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
    pub rows: Vec<RedirectEditorRow>,
    add_button_state: button::State,
}

impl RedirectEditor {
    pub fn view(&mut self, config: &Config, translator: &Translator) -> Container<Message> {
        let redirects = config.get_redirects();

        let inner = Container::new({
            self.rows
                .iter_mut()
                .enumerate()
                .fold(Column::new().padding(5).spacing(4), |parent, (i, x)| {
                    parent.push(
                        Row::new()
                            .spacing(20)
                            .push(
                                PickList::new(&mut x.pick_list, RedirectKind::ALL, Some(redirects[i].kind), move |v| {
                                    Message::SelectedRedirectKind(i, v)
                                })
                                .style(style::PickList::Primary(config.theme)),
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
                                .style(style::TextInput(config.theme))
                                .width(Length::FillPortion(3))
                                .padding(5),
                            )
                            .push(
                                Button::new(&mut x.source_browse_button_state, Icon::FolderOpen.as_text())
                                    .on_press(Message::BrowseDir(BrowseSubject::RedirectSource(i)))
                                    .style(style::Button::Primary(config.theme)),
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
                                .style(style::TextInput(config.theme))
                                .width(Length::FillPortion(3))
                                .padding(5),
                            )
                            .push(
                                Button::new(&mut x.target_browse_button_state, Icon::FolderOpen.as_text())
                                    .on_press(Message::BrowseDir(BrowseSubject::RedirectTarget(i)))
                                    .style(style::Button::Primary(config.theme)),
                            )
                            .push(
                                Button::new(&mut x.remove_button_state, Icon::RemoveCircle.as_text())
                                    .on_press(Message::EditedRedirect(EditAction::Remove(i), None))
                                    .style(style::Button::Negative(config.theme)),
                            ),
                    )
                })
                .push(
                    Button::new(&mut self.add_button_state, Icon::AddCircle.as_text())
                        .on_press(Message::EditedRedirect(EditAction::Add, None))
                        .style(style::Button::Primary(config.theme)),
                )
        })
        .style(style::Container::GameListEntry(config.theme));

        Container::new(inner)
    }
}
