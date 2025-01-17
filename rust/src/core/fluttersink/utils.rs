use glib;

// TODO: make sure that the main thread here is the same as flutter platfor main thread.
pub(crate) fn invoke_on_main_thread<F, T>(func: F) -> T
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
