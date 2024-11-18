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
    event::{self, Event},
    mouse, touch,
    widget::overlay::menu::{self, Menu},
    Border, Element, Length, Padding, Rectangle, Shadow, Size, Vector,
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

    fn layout(&self, _tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        layout(renderer, limits, self.width, self.padding, self.text_size, self.font)
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        update(
            event,
            layout,
            cursor,
            shell,
            self.on_selected.as_ref(),
            None,
            &self.options,
            || tree.state.downcast_mut::<State<T>>(),
        )
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse_interaction(layout, cursor, !self.options.is_empty())
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
        draw(renderer, theme, layout, cursor, &self.style)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
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

            Some(menu.overlay(layout.position() + translation, bounds.height))
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

/// Computes the layout of a [`PopupMenu`].
pub fn layout<Renderer>(
    renderer: &Renderer,
    limits: &layout::Limits,
    width: Length,
    padding: Padding,
    text_size: Option<f32>,
    _font: Option<Renderer::Font>,
) -> layout::Node
where
    Renderer: text::Renderer,
{
    let limits = limits.width(width).height(Length::Shrink);

    let text_size = text_size.unwrap_or_else(|| renderer.default_size().0);

    let max_width = match width {
        Length::Shrink => 10.0,
        _ => 0.0,
    };

    let size = {
        let intrinsic = Size::new(max_width + text_size + padding.left, text_size);

        limits
            .width(width)
            .shrink(padding)
            .resolve(width, Length::Shrink, intrinsic)
            .expand(padding)
    };

    layout::Node::new(Size::new(size.width, 24.0))
}

/// Processes an [`Event`] and updates the [`State`] of a [`PopupMenu`]
/// accordingly.
pub fn update<'a, T, Message>(
    event: Event,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    shell: &mut Shell<'_, Message>,
    on_selected: &dyn Fn(T) -> Message,
    selected: Option<&T>,
    options: &[T],
    state: impl FnOnce() -> &'a mut State<T>,
) -> event::Status
where
    T: PartialEq + Clone + 'a,
{
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) => {
            let state = state();

            let event_status = if state.is_open {
                // Event wasn't processed by overlay, so cursor was clicked either outside it's
                // bounds or on the drop-down, either way we close the overlay.
                state.is_open = false;

                event::Status::Captured
            } else if cursor.is_over(layout.bounds()) {
                state.is_open = !options.is_empty();
                state.hovered_option = options.iter().position(|option| Some(option) == selected);

                event::Status::Captured
            } else {
                event::Status::Ignored
            };

            if let Some(last_selection) = state.last_selection.take() {
                shell.publish((on_selected)(last_selection));

                state.is_open = false;

                event::Status::Captured
            } else {
                event_status
            }
        }
        Event::Mouse(mouse::Event::WheelScrolled { .. }) | Event::Touch(touch::Event::FingerMoved { .. }) => {
            let state = state();
            state.is_open = false;
            event::Status::Ignored
        }
        _ => event::Status::Ignored,
    }
}

/// Returns the current [`mouse::Interaction`] of a [`PopupMenu`].
pub fn mouse_interaction(layout: Layout<'_>, cursor: mouse::Cursor, usable: bool) -> mouse::Interaction {
    let bounds = layout.bounds();
    let is_mouse_over = cursor.is_over(bounds);

    if is_mouse_over && usable {
        mouse::Interaction::Pointer
    } else {
        mouse::Interaction::default()
    }
}

/// Draws a [`PopupMenu`].
pub fn draw<Theme, Renderer>(
    renderer: &mut Renderer,
    theme: &Theme,
    layout: Layout<'_>,
    cursor: mouse::Cursor,
    style: &<Theme as Catalog>::Class<'_>,
) where
    Theme: Catalog,
    Renderer: text::Renderer<Font = iced::Font>,
{
    let bounds = layout.bounds();
    let is_mouse_over = cursor.is_over(bounds);

    let status = if is_mouse_over { Status::Hovered } else { Status::Active };

    let style = <Theme as Catalog>::style(theme, style, status);

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
            horizontal_alignment: alignment::Horizontal::Center,
            vertical_alignment: alignment::Vertical::Center,
            line_height: text::LineHeight::default(),
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::Word,
        },
        bounds.center(),
        style.text_color,
        bounds,
    );
}
