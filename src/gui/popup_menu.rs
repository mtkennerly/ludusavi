// Based on Iced 0.5.0's PickList:
// https://github.com/iced-rs/iced/blob/0.5.0/native/src/widget/pick_list.rs

//! Display a dropdown list of selectable values.
use std::borrow::Cow;

pub use iced::widget::pick_list::{Catalog, Status};
use iced::{
    advanced::{
        self, layout, overlay, renderer, text,
        widget::tree::{self, Tree},
        Clipboard, Layout, Shell, Widget,
    },
    alignment,
    event::Event,
    mouse, touch,
    widget::overlay::menu::{self, Menu},
    window, Border, Element, Length, Padding, Rectangle, Shadow, Size, Vector,
};

/// A widget for selecting a single value from a list of options.
#[allow(missing_debug_implementations)]
pub struct PopupMenu<'a, T, Message, Theme = crate::gui::style::Theme, Renderer = iced::Renderer>
where
    [T]: ToOwned<Owned = Vec<T>>,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    on_selected: Box<dyn Fn(T) -> Message + 'a>,
    options: Cow<'a, [T]>,
    width: Length,
    padding: Padding,
    text_size: Option<f32>,
    font: Option<Renderer::Font>,
    style: <Theme as Catalog>::Class<'a>,
    menu_style: <Theme as menu::Catalog>::Class<'a>,
    last_status: Option<Status>,
}

impl<'a, T: 'a, Message, Theme, Renderer> PopupMenu<'a, T, Message, Theme, Renderer>
where
    T: ToString + Eq,
    [T]: ToOwned<Owned = Vec<T>>,
    Theme: Catalog,
    Renderer: text::Renderer,
{
    /// The default padding of a [`PopupMenu`].
    pub const DEFAULT_PADDING: Padding = Padding::new(5.0);

    /// Creates a new [`PopupMenu`] with the given list of options, the current
    /// selected value, and the message to produce when an option is selected.
    pub fn new(options: impl Into<Cow<'a, [T]>>, on_selected: impl Fn(T) -> Message + 'a) -> Self {
        Self {
            on_selected: Box::new(on_selected),
            options: options.into(),
            width: Length::Shrink,
            text_size: None,
            padding: Self::DEFAULT_PADDING,
            font: None,
            style: <Theme as Catalog>::default(),
            menu_style: <Theme as menu::Catalog>::default(),
            last_status: None,
        }
    }

    /// Sets the width of the [`PopupMenu`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the style of the [`PopupMenu`].
    pub fn class(mut self, style: impl Into<<Theme as Catalog>::Class<'a>>) -> Self {
        self.style = style.into();
        self
    }

    /// Sets the style of the [`Menu`].
    pub fn menu_class(mut self, style: impl Into<<Theme as menu::Catalog>::Class<'a>>) -> Self {
        self.menu_style = style.into();
        self
    }
}

impl<'a, T: 'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PopupMenu<'a, T, Message, Theme, Renderer>
where
    T: Clone + ToString + Eq + 'static,
    [T]: ToOwned<Owned = Vec<T>>,
    Message: 'a,
    Theme: Catalog,
    Renderer: text::Renderer<Font = iced::Font> + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<T>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<T>::new())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: Length::Shrink,
        }
    }

    fn layout(&mut self, _tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(Length::Shrink);

        let text_size = self.text_size.unwrap_or_else(|| renderer.default_size().0);

        let max_width = match self.width {
            Length::Shrink => 10.0,
            _ => 0.0,
        };

        let size = {
            let intrinsic = Size::new(max_width + text_size + self.padding.left, text_size);

            limits
                .width(self.width)
                .shrink(self.padding)
                .resolve(self.width, Length::Shrink, intrinsic)
                .expand(self.padding)
        };

        layout::Node::new(Size::new(size.width, 24.0))
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<T>>();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if state.is_open {
                    // Event wasn't processed by overlay, so cursor was clicked either outside its
                    // bounds or on the drop-down, either way we close the overlay.
                    state.is_open = false;

                    shell.capture_event();
                } else if cursor.is_over(layout.bounds()) {
                    state.is_open = !self.options.is_empty();

                    shell.capture_event();
                }

                if let Some(last_selection) = state.last_selection.take() {
                    shell.publish((self.on_selected.as_ref())(last_selection));

                    state.is_open = false;

                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { .. }) | Event::Touch(touch::Event::FingerMoved { .. }) => {
                state.is_open = false;
            }
            _ => {}
        }

        let status = {
            let is_hovered = cursor.is_over(layout.bounds());

            if state.is_open {
                Status::Opened { is_hovered }
            } else if is_hovered {
                Status::Hovered
            } else {
                Status::Active
            }
        };

        if let Event::Window(window::Event::RedrawRequested(_now)) = event {
            self.last_status = Some(status);
        } else if self.last_status.is_some_and(|last_status| last_status != status) {
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        if is_mouse_over && !self.options.is_empty() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let is_mouse_over = cursor.is_over(bounds);

        let status = if is_mouse_over { Status::Hovered } else { Status::Active };

        let style = <Theme as Catalog>::style(theme, &self.style, status);

        if is_mouse_over {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        color: style.border.color,
                        width: style.border.width,
                        radius: style.border.radius,
                    },
                    shadow: Shadow {
                        color: iced::Color::BLACK,
                        offset: Vector::ZERO,
                        blur_radius: 0.0,
                    },
                    snap: true,
                },
                style.background,
            );
        }

        let icon_size = 0.5;
        renderer.fill_text(
            advanced::Text {
                content: crate::gui::icon::Icon::MoreVert.as_char().to_string(),
                font: crate::gui::font::ICONS,
                size: (bounds.height * icon_size * 1.5).into(),
                bounds: Size {
                    width: bounds.width,
                    height: bounds.height,
                },
                align_x: advanced::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                line_height: text::LineHeight::default(),
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::Word,
            },
            bounds.center(),
            style.text_color,
            bounds,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State<T>>();

        if state.is_open {
            let bounds = layout.bounds();

            let mut menu = Menu::new(
                &mut state.menu,
                &self.options,
                &mut state.hovered_option,
                |option| {
                    state.is_open = false;

                    (self.on_selected)(option)
                },
                None,
                &self.menu_style,
            )
            .width(150.0)
            .padding(self.padding)
            .font(self.font.unwrap_or_else(|| renderer.default_font()))
            .text_shaping(text::Shaping::Advanced);

            if let Some(text_size) = self.text_size {
                menu = menu.text_size(text_size);
            }

            Some(menu.overlay(
                layout.position() + translation,
                *viewport,
                bounds.height,
                Length::Shrink,
            ))
        } else {
            None
        }
    }
}

impl<'a, T: 'a, Message, Theme, Renderer> From<PopupMenu<'a, T, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Clone + ToString + Eq + 'static,
    [T]: ToOwned<Owned = Vec<T>>,
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer<Font = iced::Font> + 'a,
{
    fn from(pick_list: PopupMenu<'a, T, Message, Theme, Renderer>) -> Self {
        Self::new(pick_list)
    }
}

/// The local state of a [`PopupMenu`].
#[derive(Debug)]
pub struct State<T> {
    menu: menu::State,
    is_open: bool,
    hovered_option: Option<usize>,
    last_selection: Option<T>,
}

impl<T> State<T> {
    /// Creates a new [`State`] for a [`PopupMenu`].
    pub fn new() -> Self {
        Self {
            menu: menu::State::default(),
            is_open: bool::default(),
            hovered_option: Option::default(),
            last_selection: Option::default(),
        }
    }
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self::new()
    }
}
