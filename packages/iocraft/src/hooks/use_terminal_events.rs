use crate::{ComponentUpdater, Hook, Hooks, TerminalEvent, TerminalEvents};
use futures::stream::Stream;
use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

/// `UseTerminalEvents` is a hook that allows you to listen for user input such as key strokes.
///
/// # Example
///
/// ```
/// # use iocraft::prelude::*;
/// # use unicode_width::UnicodeWidthStr;
/// const AREA_WIDTH: u32 = 80;
/// const AREA_HEIGHT: u32 = 11;
/// const FACE: &str = "👾";
///
/// #[component]
/// fn Example(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
///     let mut system = hooks.use_context_mut::<SystemContext>();
///     let x = hooks.use_state(|| 0);
///     let y = hooks.use_state(|| 0);
///     let should_exit = hooks.use_state(|| false);
///
///     hooks.use_terminal_events({
///         move |event| match event {
///             TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
///                 match code {
///                     KeyCode::Char('q') => should_exit.set(true),
///                     KeyCode::Up => y.set((y.get() as i32 - 1).max(0) as _),
///                     KeyCode::Down => y.set((y.get() + 1).min(AREA_HEIGHT - 1)),
///                     KeyCode::Left => x.set((x.get() as i32 - 1).max(0) as _),
///                     KeyCode::Right => x.set((x.get() + 1).min(AREA_WIDTH - FACE.width() as u32)),
///                     _ => {}
///                 }
///             }
///             _ => {}
///         }
///     });
///
///     if should_exit.get() {
///         system.exit();
///     }
///
///     element! {
///         Box(
///             flex_direction: FlexDirection::Column,
///             padding: 2,
///             align_items: AlignItems::Center
///         ) {
///             Text(content: "Use arrow keys to move. Press \"q\" to exit.")
///             Box(
///                 border_style: BorderStyle::Round,
///                 border_color: Color::Green,
///                 height: AREA_HEIGHT + 2,
///                 width: AREA_WIDTH + 2,
///             ) {
///                 #(if should_exit.get() {
///                     element! {
///                         Box(
///                             width: 100pct,
///                             height: 100pct,
///                             justify_content: JustifyContent::Center,
///                             align_items: AlignItems::Center,
///                         ) {
///                             Text(content: format!("Goodbye! {}", FACE))
///                         }
///                     }
///                 } else {
///                     element! {
///                         Box(
///                             padding_left: x.get(),
///                             padding_top: y.get(),
///                         ) {
///                             Text(content: FACE)
///                         }
///                     }
///                 })
///             }
///         }
///     }
/// }
/// ```
pub trait UseTerminalEvents {
    /// Defines a callback to be invoked whenever a terminal event occurs.
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static;
}

impl UseTerminalEvents for Hooks<'_, '_> {
    fn use_terminal_events<F>(&mut self, f: F)
    where
        F: FnMut(TerminalEvent) + Send + 'static,
    {
        self.use_hook(move || UseTerminalEventsImpl {
            events: None,
            f: Box::new(f),
        });
    }
}

struct UseTerminalEventsImpl {
    events: Option<TerminalEvents>,
    f: Box<dyn FnMut(TerminalEvent) + Send + 'static>,
}

impl Hook for UseTerminalEventsImpl {
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        while let Some(Poll::Ready(Some(event))) = self
            .events
            .as_mut()
            .map(|events| pin!(events).poll_next(cx))
        {
            (self.f)(event);
        }
        Poll::Pending
    }

    fn post_component_update(&mut self, updater: &mut ComponentUpdater) {
        if self.events.is_none() {
            self.events = updater.terminal_events();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use futures::stream::{self, StreamExt};
    use macro_rules_attribute::apply;
    use smol_macros::test;

    #[component]
    fn MyComponent(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
        let mut system = hooks.use_context_mut::<SystemContext>();
        let should_exit = hooks.use_state(|| false);
        hooks.use_terminal_events(move |_event| {
            should_exit.set(true);
        });

        if should_exit.get() {
            system.exit();
            element!(Text(content:"received event")).into_any()
        } else {
            element!(Box).into_any()
        }
    }

    #[apply(test!)]
    async fn test_use_terminal_events() {
        let canvases: Vec<_> = element!(MyComponent)
            .mock_terminal_render_loop(MockTerminalConfig::with_events(stream::iter(vec![
                TerminalEvent::Key(KeyEvent {
                    code: KeyCode::Char('f'),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                }),
            ])))
            .collect()
            .await;
        let actual = canvases.iter().map(|c| c.to_string()).collect::<Vec<_>>();
        let expected = vec!["", "received event\n"];
        assert_eq!(actual, expected);
    }
}
