use parking_lot::ReentrantMutex;
use std::env::{current_dir, set_current_dir};
use std::sync::{Arc, OnceLock};
use std::{env, io};

/// Prevents race conditions in multithreaded tests
static WITH_DIRECTORY_MUTEX: OnceLock<Arc<ReentrantMutex<()>>> = OnceLock::new();

pub fn with_directory<F, T>(directory: Option<String>, closure: F) -> io::Result<T>
where
    F: FnOnce() -> io::Result<T>,
{
    let _guard = WITH_DIRECTORY_MUTEX
        .get_or_init(|| Arc::new(ReentrantMutex::new(())))
        .lock();

    let previous_directory = current_dir()?.canonicalize()?;
    if let Some(directory) = directory {
        set_current_dir(directory)?;
    }
    let result = closure();
    set_current_dir(previous_directory)?;
    result
}

pub fn init_logging(verbose: bool) {
    if verbose {
        env::set_var("RUST_LOG", "debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
}
