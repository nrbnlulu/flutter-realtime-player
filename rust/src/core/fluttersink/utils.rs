use irondash_run_loop::RunLoop;
use log::error;

pub(crate) fn is_fl_main_thread() -> bool {
    RunLoop::is_main_thread().unwrap_or(false)
}

pub trait LogErr<T> {
    fn log_err(self) -> Option<T>;
}

impl<T, E> LogErr<T> for Result<T, E>
where
    E: std::fmt::Debug,
{
    fn log_err(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                error!("Error: {:?}", err);
                None
            }
        }
    }
}
