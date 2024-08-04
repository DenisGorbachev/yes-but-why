//! The [`helpful::Error`](Error) is an upgraded version of [`anyhow::Error`].
//! It provides additional information from the current span trace.
//! This information can be used to diagnose the root cause of the error, which simplifies debugging & provides helpful error messages to the users.
//!
//! Features:
//!
//! * [x] Can be propagated up the call stack, just like [`anyhow::Error`]
//! * [x] Can be constructed from external error types, just like [`anyhow::Error`]
//! * [x] Captures the current tracing span, just like [`tracing_error::TracedError<E>`]
//!
//! Benefits:
//!
//! * Provides a detailed span trace to the user (which makes it easier to diagnose the root cause of the error)
//! * Provides a detailed span trace to the developer (which simplifies debugging)
//!
//! Advantages over [`anyhow::Error`]:
//!
//! * Provides additional information from the current span trace
//!
//! Advantages over [`tracing_error::TracedError<E>`]:
//!
//! * Can be propagated up the call stack with `?` operator (no explicit conversion needed). This is because [`Error`] doesn't have any generic arguments, so you can call any function that returns a `Result<T, TracingError>` and apply a `?` operator to the result. By contrast, [`tracing_error::TracedError<E>`] is generic over `E`, so you can't compose the functions that return different `Result<T, TracedError<E>>`.
//!
//! [`anyhow::Error`]: https://docs.rs/anyhow/latest/anyhow/struct.Error.html
//! [`tracing_error::TracedError<E>`]: https://docs.rs/tracing-error/latest/tracing_error/struct.TracedError.html
//!
//! # Setup
//!
//! * Initialize the tracing subscriber in `main`
//! * Ensure the default level is set to `Level::INFO` (or modify your `instrument` attributes to collect the data at another level)
//!
//! # **Important setup note**
//!
//! If you don't see any tracing spans in the error message, check your tracing subscriber configuration. Here's an example of a correct configuration:
//!
//! ```
//! fn main() {
//!     init_tracing_subscriber();
//!     // your code here
//! }
//!
//! fn init_tracing_subscriber() {
//!    use tracing_subscriber::util::SubscriberInitExt;
//!    use tracing::level_filters::LevelFilter;
//!    use tracing_error::ErrorLayer;
//!    use tracing_subscriber::layer::SubscriberExt;
//!    let env_filter = tracing_subscriber::EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy();
//!    let subscriber = tracing_subscriber::fmt()
//!        .with_env_filter(env_filter)
//!        // .with_max_level(tracing::Level::TRACE) // Set the maximum log level to TRACE
//!        .finish()
//!        .with(ErrorLayer::default());
//!    // dbg!(&subscriber);
//!    subscriber.init();
//!}
//! ```
//!

use std::backtrace::{Backtrace, BacktraceStatus};
use std::error::Error as StdError;
use std::fmt;
use std::fmt::{Debug, Display};
use std::process::{ExitCode, Termination};
use std::result::Result as StdResult;

use tracing_error::SpanTrace;

/// This type doesn't implement the `Error` trait because it conflicts with a blanket `From<E>` implementation (which allows converting any error to this type). This is the same reason why `anyhow::Error` doesn't implement `Error`.
#[derive(Debug)]
pub struct Error {
    pub source: Box<dyn StdError + Send + Sync + 'static>,
    pub span_trace: SpanTrace,
    pub backtrace: Backtrace,
}

impl Error {
    pub fn new<E: StdError + Send + Sync + 'static>(source: E) -> Self {
        Self {
            source: Box::new(source),
            span_trace: SpanTrace::capture(),
            backtrace: Backtrace::capture(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Error: ")?;
        Display::fmt(self.source.as_ref(), f)?;
        f.pad("\n\n")?;
        f.pad("Span trace:\n")?;
        Display::fmt(&self.span_trace, f)?;
        if let BacktraceStatus::Captured = self.backtrace.status() {
            f.pad("\n\n")?;
            f.pad("Backtrace:\n")?;
            Display::fmt(&self.backtrace, f)?;
        }
        Ok(())
    }
}

impl<E: StdError + Send + Sync + 'static> From<E> for Error {
    fn from(source: E) -> Self {
        Self::new(source)
    }
}

pub type Result<T = ()> = StdResult<T, Error>;

pub trait Traced {
    type Output;

    fn traced(self) -> Self::Output;
}

impl<T, E: Into<Error>> Traced for StdResult<T, E> {
    type Output = StdResult<T, Error>;

    fn traced(self) -> Self::Output {
        self.map_err(Into::into)
    }
}

pub enum MainResult<T = (), E = Error> {
    Ok(T),
    Err(E),
}

impl<T, E> From<StdResult<T, E>> for MainResult<T, E> {
    fn from(value: StdResult<T, E>) -> Self {
        match value {
            Ok(value) => MainResult::Ok(value),
            Err(error) => MainResult::Err(error),
        }
    }
}

impl<T: Termination, E: Display> Termination for MainResult<T, E> {
    fn report(self) -> ExitCode {
        match self {
            MainResult::Ok(value) => value.report(),
            MainResult::Err(error) => {
                // TODO: attempt_print_to_stderr is private, need a workaround
                // std::io::attempt_print_to_stderr(format_args_nl!("Error: {err:?}"));
                eprintln!("{}", error);
                ExitCode::FAILURE
            }
        }
    }
}
