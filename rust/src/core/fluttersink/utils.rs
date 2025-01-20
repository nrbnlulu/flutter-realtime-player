use glib;
use irondash_run_loop::RunLoop;
use log::debug;

pub(crate) fn invoke_on_gs_main_thread<F, T>(func: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let context = glib::MainContext::default();

    let (send, recv) = flume::bounded(1);
    context.invoke(move || {
        send.send(func()).expect("Somehow we dropped the receiver");
    });
    recv.recv().expect("Somehow we dropped the sender")
}

/// Inboke the given function on the flutter engine main thread.
pub(crate) fn invoke_on_platform_main_thread<F, T>(func: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    if RunLoop::is_main_thread().unwrap_or(false) {
        debug!("`invoke_on_platform_main_thread()` was called on the main thread");
        return func();
    }
    let (send, recv) = flume::bounded(1);

    RunLoop::sender_for_main_thread().unwrap().send(move || {
        send.send(func()).expect("Somehow we dropped the receiver");
    });
    recv.recv().expect("Somehow we dropped the sender")
}

pub(crate) fn make_element(factory_name: &str, name: Option<&str>) -> anyhow::Result<gst::Element> {
    gst::ElementFactory::make_with_name(factory_name, name)
        .map_err(|_| anyhow::anyhow!("Failed to create element"))
}
