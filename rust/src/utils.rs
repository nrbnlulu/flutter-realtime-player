use log::error;

/// GStreamer plugin registration return type: 1 = success, 0 = failure.
#[allow(unused)]
pub(crate) type GstBool = i32;

#[allow(unused)]
pub(crate) fn is_gst_result_ok(result: GstBool) -> bool {
    result == 1
}

pub trait LogErr<T> {
    fn log_err(self) -> Option<T>;
}

impl<T, E> LogErr<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    #[track_caller]
    fn log_err(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                error!("Error: {}", err);
                None
            }
        }
    }
}
