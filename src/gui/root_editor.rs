use crate::{
    config::Config,
    gui::{
        common::{BrowseSubject, EditAction},
        common::{Message, OngoingOperation},
        icon::Icon,
        style,
    },
    lang::Translator,
    manifest::Store,
    shortcuts::TextHistory,
};

use iced::{
    button, pick_list, scrollable, text_input, Button, Container, Length, PickList, Row, Scrollable, Space, Text,
    TextInput,
};

#[derive(Default)]
pub struct RootEditorRow {
    button_state: button::State,
    browse_button_state: button::State,
    pub text_state: text_input::State,
    pub text_history: TextHistory,
    pick_list: pick_list::State<Store>,
}

impl RootEditorRow {
    pub fn new(initial_text: &str) -> Self {
        Self {
            text_history: TextHistory::new(initial_text, 100),
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct RootEditor {
    scroll: scrollable::State,
    pub rows: Vec<RootEditorRow>,
}

impl RootEditor {
    pub fn view(
        &mut self,
        config: &Config,
        translator: &Translator,
        operation: &Option<OngoingOperation>,
    ) -> Container<Message> {
        let roots = config.roots.clone();
        if roots.is_empty() {
            Container::new(Text::new(translator.no_roots_are_configured()))
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
                                            .on_press(Message::EditedRoot(EditAction::Remove(i)))
                                            .style(style::Button::Negative),
                                    )
                                    .push(
                                        TextInput::new(&mut x.text_state, "", &roots[i].path.raw(), move |v| {
                                            Message::EditedRoot(EditAction::Change(i, v))
                                        })
                                        .width(Length::FillPortion(3))
                                        .padding(5),
                                    )
                                    .push(PickList::new(
                                        &mut x.pick_list,
                                        vec![
                                            Store::Epic,
                                            Store::Gog,
                                            Store::GogGalaxy,
                                            Store::Microsoft,
                                            Store::Origin,
                                            Store::Steam,
                                            Store::Uplay,
                                            Store::Other,
                                        ],
                                        Some(roots[i].store),
                                        move |v| Message::SelectedRootStore(i, v),
                                    ))
                                    .push(
                                        Button::new(&mut x.browse_button_state, Icon::FolderOpen.as_text())
                                            .on_press(match operation {
                                                None => Message::BrowseDir(BrowseSubject::Root(i)),
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
