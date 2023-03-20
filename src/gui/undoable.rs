use iced::keyboard::KeyCode;
use iced_native::{
    event::{self, Event},
    layout, mouse, overlay, renderer,
    widget::{Operation, Tree},
    Clipboard, Element, Layout, Length, Point, Rectangle, Shell, Widget,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Undo,
    Redo,
}

#[allow(missing_debug_implementations)]
pub struct Undoable<'a, Message, Renderer, F>
where
    Message: Clone,
    F: Fn(Action) -> Message + 'a,
{
    content: Element<'a, Message, Renderer>,
    on_change: F,
}

impl<'a, Message, Renderer, F> Undoable<'a, Message, Renderer, F>
where
    Message: Clone,
    F: Fn(Action) -> Message + 'a,
{
    pub fn new<T>(content: T, on_change: F) -> Self
    where
        T: Into<Element<'a, Message, Renderer>>,
    {
        Self {
            content: content.into(),
            on_change,
        }
    }
}

impl<'a, Message, Renderer, F> Widget<Message, Renderer> for Undoable<'a, Message, Renderer, F>
where
    Message: Clone,
    Renderer: iced_native::Renderer,
    F: Fn(Action) -> Message + 'a,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content))
    }

    fn width(&self) -> Length {
        self.content.as_widget().width()
    }

    fn height(&self) -> Length {
        self.content.as_widget().height()
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget().layout(renderer, limits)
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<Message>,
    ) {
        self.content
            .as_widget()
            .operate(&mut tree.children[0], layout, renderer, operation)
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key_code, modifiers }) = event {
            let focused = tree.children[0]
                .state
                .downcast_ref::<iced_native::widget::text_input::State>()
                .is_focused();
            if focused {
                match (key_code, modifiers.command(), modifiers.shift()) {
                    (KeyCode::Z, true, false) => {
                        shell.publish((self.on_change)(Action::Undo));
                        return event::Status::Captured;
                    }
                    (KeyCode::Y, true, false) | (KeyCode::Z, true, true) => {
                        shell.publish((self.on_change)(Action::Redo));
                        return event::Status::Captured;
                    }
                    _ => (),
                };
            }
        }

        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor_position,
            renderer,
            clipboard,
            shell,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(&tree.children[0], layout, cursor_position, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor_position,
            viewport,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer)
    }
}

impl<'a, Message, Renderer, F> From<Undoable<'a, Message, Renderer, F>> for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: iced_native::Renderer + 'a,
    F: Fn(Action) -> Message + 'a,
{
    fn from(undoable: Undoable<'a, Message, Renderer, F>) -> Self {
        Self::new(undoable)
    }
}
