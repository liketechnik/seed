use super::{App, CmdHandle, RenderTimestampDelta, StreamHandle, SubHandle, UndefinedGMsg};
use crate::virtual_dom::View;
use futures::stream::Stream;
use std::{any::Any, future::Future};

// @TODO: Add links to doc comment once https://github.com/rust-lang/rust/issues/43466 is resolved
// or use nightly rustdoc. Applicable to the entire code base.

pub mod container;
pub mod proxy;

pub use container::OrdersContainer;
pub use proxy::OrdersProxy;

pub trait Orders<Ms: 'static, GMs = UndefinedGMsg> {
    type AppMs: 'static;
    type Mdl: 'static;
    type ElC: View<Self::AppMs> + 'static;

    /// Automatically map message type. It allows you to pass `Orders` into child module.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///Msg::Child(child_msg) => {
    ///    child::update(child_msg, &mut model.child, &mut orders.proxy(Msg::Child));
    ///}
    /// ```
    fn proxy<ChildMs: 'static>(
        &mut self,
        f: impl FnOnce(ChildMs) -> Ms + 'static + Clone,
    ) -> OrdersProxy<ChildMs, Self::AppMs, Self::Mdl, Self::ElC, GMs>;

    /// Schedule web page rerender after model update. It's the default behaviour.
    fn render(&mut self) -> &mut Self;

    /// Force web page to rerender immediately after model update.
    fn force_render_now(&mut self) -> &mut Self;

    /// Don't rerender web page after model update.
    fn skip(&mut self) -> &mut Self;

    /// Notify all subscription handlers which listen for messages with the `message`'s type.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///orders.notify(counter::DoReset);
    ///orders.notify("Hello!");
    /// ...
    ///orders.subscribe(Msg::Reset);  // `Msg::Reset(counter::DoReset)`
    ///orders.subscribe(|greeting: &'static str| { log!(greeting); Msg::NoOp });
    /// ```
    ///
    /// _Note:_: All notifications are pushed to the queue - i.e. `update` function is NOT called immediately.
    fn notify(&mut self, message: impl Any + Clone) -> &mut Self;

    /// Invoke function `update` with the given `msg`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///orders.msg(Msg::Increment);
    /// ```
    ///
    /// _Note:_: All `msg`s are pushed to the queue - i.e. `update` function is NOT called immediately.
    fn send_msg(&mut self, msg: Ms) -> &mut Self;

    /// Execute given future `cmd` and send its output (`Msg`) to `update` function.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///orders.perform_cmd(cmds::timeout(2000, || Msg::OnTimeout)));
    ///orders.perform_cmd(async { log!("Hello!"); Msg::NoOp });
    /// ```
    ///
    /// _Note:_: Use the alternative `perform_cmd_with_handle` to control `cmd`'s lifetime.
    fn perform_cmd(&mut self, cmd: impl Future<Output = Ms> + 'static) -> &mut Self;

    /// Execute given future `cmd` and send its output (`Msg`) to `update` function.
    /// - Returns `CmdHandle` that you should save to your `Model`.
    ///   The `cmd` is aborted on the handle drop.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///let timeout_handle = orders.perform_cmd_with_handle(cmds::timeout(2000, || Msg::OnTimeout)));
    ///let cmd_handle = orders.perform_cmd_with_handle(async { log!("Hello!"); Msg::NoOp });
    /// ```
    #[must_use = "cmd is aborted on its handle drop"]
    fn perform_cmd_with_handle(&mut self, cmd: impl Future<Output = Ms> + 'static) -> CmdHandle;

    /// Similar to `send_msg`, but calls function `sink` with the given global message.
    fn send_g_msg(&mut self, g_msg: GMs) -> &mut Self;

    /// Similar to `perform_cmd`, but result is send to function `sink`.
    fn perform_g_cmd(&mut self, g_cmd: impl Future<Output = GMs> + 'static) -> &mut Self;

    /// Similar to `perform_g_cmd`, but result is send to function `sink`.
    /// - Returns `CmdHandle` that you should save to your `Model`.
    ///   `cmd` is aborted on the handle drop.
    #[must_use = "cmd is aborted on its handle drop"]
    fn perform_g_cmd_with_handle(
        &mut self,
        g_cmd: impl Future<Output = GMs> + 'static,
    ) -> CmdHandle;

    /// Get app instance. Cloning is cheap because `App` contains only `Rc` fields.
    fn clone_app(&self) -> App<Self::AppMs, Self::Mdl, Self::ElC, GMs>;

    /// Get function which maps module's `Msg` to app's (root's) one.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///let (app, msg_mapper) = (orders.clone_app(), orders.msg_mapper());
    ///app.update(msg_mapper(Msg::AMessage));
    /// ```
    fn msg_mapper(&self) -> Box<dyn Fn(Ms) -> Self::AppMs>;

    /// Register the callback that will be executed after the next render.
    ///
    /// Callback's only parameter is `Option<RenderTimestampDelta>` - the difference between
    /// the old render timestamp and the new one.
    /// The parameter has value `None` if it's the first rendering.
    ///
    /// - It's useful when you want to use DOM API or make animations.
    /// - You can call this function multiple times - callbacks will be executed in the same order.
    ///
    /// _Note:_ [performance.now()](https://developer.mozilla.org/en-US/docs/Web/API/Performance/now)
    ///  is used under the hood to get timestamps.
    fn after_next_render(
        &mut self,
        callback: impl FnOnce(Option<RenderTimestampDelta>) -> Ms + 'static,
    ) -> &mut Self;

    /// Subscribe for messages with the `handler`s input type.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///orders.subscribe(Msg::Reset);  // `Msg::Reset(counter::DoReset)`
    ///orders.subscribe(|greeting: &'static str| { log!(greeting); Msg::NoOp });
    ///orders.subscribe(Msg::UrlChanged)  // `update(... Msg::UrlChanged(subs::UrlChanged(url)) =>`
    /// ...
    ///orders.notify(counter::DoReset);
    ///orders.notify("Hello!");
    /// ```
    ///
    /// _Note:_: Use the alternative `subscribe_with_handle` to control `sub`'s lifetime.
    fn subscribe<SubMs: 'static + Clone>(
        &mut self,
        handler: impl FnOnce(SubMs) -> Ms + Clone + 'static,
    ) -> &mut Self;

    /// Subscribe for messages with the `handler`s input type.
    /// - Returns `SubHandle` that you should save to your `Model`.
    ///   The `sub` is cancelled on the handle drop.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///let sub_handle = orders.subscribe_with_handle(Msg::Reset);  // `Msg::Reset(counter::DoReset)`
    ///orders.subscribe_with_handle(|greeting: &'static str| { log!(greeting); Msg::NoOp });
    ///let url_changed_handle = orders.subscribe_with_handle(Msg::UrlChanged)  // `update(... Msg::UrlChanged(subs::UrlChanged(url)) =>`
    /// ...
    ///orders.notify(counter::DoReset);
    ///orders.notify("Hello!");
    /// ```
    #[must_use = "subscription is cancelled on its handle drop"]
    fn subscribe_with_handle<SubMs: 'static + Clone>(
        &mut self,
        handler: impl FnOnce(SubMs) -> Ms + Clone + 'static,
    ) -> SubHandle;

    /// Stream `Msg`s.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///orders.stream(streams::interval(1000, || Msg::OnTick)));
    ///orders.stream(streams::window_event(Ev::Resize, |_| Msg::OnResize));
    /// ```
    ///
    /// _Note:_: Use the alternative `stream_with_handle` to control `stream`'s lifetime.
    fn stream(&mut self, stream: impl Stream<Item = Ms> + 'static) -> &mut Self;

    /// Stream `Msg`s.
    /// - Returns `StreamHandle` that you should save to your `Model`.
    ///   The `stream` is cancelled on the handle drop.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    ///let timer_handler = orders.stream_with_handle(streams::interval(1000, || Msg::OnTick)));
    ///let stream_handler = orders.stream_with_handle(streams::window_event(Ev::Resize, |_| Msg::OnResize));
    /// ```
    #[must_use = "stream is stopped on its handle drop"]
    fn stream_with_handle(&mut self, stream: impl Stream<Item = Ms> + 'static) -> StreamHandle;
}
