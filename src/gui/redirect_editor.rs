use crate::{
    config::{Config, RedirectKind},
    gui::{
        button,
        common::{BrowseSubject, Message, TextHistories, UndoSubject},
        style,
        widget::{Column, Container, PickList, Row},
    },
};

#[derive(Default)]
pub struct RedirectEditor {}

impl RedirectEditor {
    pub fn view<'a>(config: &Config, histories: &TextHistories) -> Container<'a> {
        let redirects = config.get_redirects();

        let inner = Container::new({
            config
                .redirects
                .iter()
                .enumerate()
                .fold(Column::new().padding(5).spacing(4), |parent, (i, _)| {
                    parent.push(
                        Row::new()
                            .spacing(20)
                            .push(button::move_up(|x| Message::EditedRedirect(x, None), i))
                            .push(button::move_down(
                                |x| Message::EditedRedirect(x, None),
                                i,
                                config.redirects.len(),
                            ))
                            .push(
                                PickList::new(RedirectKind::ALL, Some(redirects[i].kind), move |v| {
                                    Message::SelectedRedirectKind(i, v)
                                })
                                .style(style::PickList::Primary),
                            )
                            .push(histories.input(UndoSubject::RedirectSource(i)))
                            .push(button::open_folder(BrowseSubject::RedirectSource(i)))
                            .push(histories.input(UndoSubject::RedirectTarget(i)))
                            .push(button::open_folder(BrowseSubject::RedirectTarget(i)))
                            .push(button::remove(|x| Message::EditedRedirect(x, None), i)),
                    )
                })
                .push(button::add(|x| Message::EditedRedirect(x, None)))
        })
        .style(style::Container::GameListEntry);

        Container::new(inner)
    }
}
