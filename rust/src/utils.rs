use log::error;

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
