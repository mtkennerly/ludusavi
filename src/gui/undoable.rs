use iced::{
    advanced::{
        layout, renderer,
        widget::{Operation, Tree},
        Clipboard, Layout, Shell, Widget,
    },
    event::Event,
    keyboard::Key,
    mouse, overlay, Element, Length, Rectangle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Undo,
    Redo,
}

#[allow(missing_debug_implementations)]
pub struct Undoable<'a, Message, Theme, Renderer, F>
where
    Message: Clone,
    F: Fn(Action) -> Message + 'a,
{
    content: Element<'a, Message, Theme, Renderer>,
    on_change: F,
}

impl<'a, Message, Theme, Renderer, F> Undoable<'a, Message, Theme, Renderer, F>
where
    Message: Clone,
    F: Fn(Action) -> Message + 'a,
{
    pub fn new<T>(content: T, on_change: F) -> Self
    where
        T: Into<Element<'a, Message, Theme, Renderer>>,
    {
        Self {
            content: content.into(),
            on_change,
        }
    }
}

impl<'a, Message, Theme, Renderer, F> Widget<Message, Theme, Renderer> for Undoable<'a, Message, Theme, Renderer, F>
where
    Message: Clone,
    Renderer: iced::advanced::text::Renderer,
    F: Fn(Action) -> Message + 'a,
{
    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree)
    }

    fn size(&self) -> iced::Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> iced::Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) = &event {
            let focused = tree
                .state
                .downcast_ref::<iced::widget::text_input::State<Renderer::Paragraph>>()
                .is_focused();
            if focused {
                match (key.as_ref(), modifiers.command(), modifiers.shift()) {
                    (Key::Character("z"), true, false) => {
                        shell.publish((self.on_change)(Action::Undo));
                        shell.capture_event();
                        return;
                    }
                    (Key::Character("y"), true, false) | (Key::Character("z"), true, true) => {
                        shell.publish((self.on_change)(Action::Redo));
                        shell.capture_event();
                        return;
                    }
                    _ => (),
                };
            }
        }

        self.content
            .as_widget_mut()
            .update(tree, event, layout, cursor, renderer, clipboard, shell, viewport)
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor_position: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor_position, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer, F> From<Undoable<'a, Message, Theme, Renderer, F>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: iced::advanced::text::Renderer + 'a,
    F: Fn(Action) -> Message + 'a,
{
    fn from(undoable: Undoable<'a, Message, Theme, Renderer, F>) -> Self {
        Self::new(undoable)
    }
}
