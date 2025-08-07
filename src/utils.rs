use std::time::{Duration, Instant};

pub fn sort_by_key_ref<T, B, F>(slice: &mut [T], mut f: F)
where
    F: FnMut(&T) -> &B,
    B: Ord,
{
    slice.sort_by(|a, b| f(a).cmp(f(b)))
}

/// Executes f and measures how long it took.
async fn measure_async<F, Fut, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let start = Instant::now();
    let res = f().await;
    let elapsed = start.elapsed();
    (res, elapsed)
}

/// Executes f and measures how long it took.
fn measure<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let res = f();
    let elapsed = start.elapsed();
    (res, elapsed)
}

/// Executes f and measures how long it took.
pub async fn log_execution_time_async<F, Fut, T>(op_description: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let (res, elapsed_time) = measure_async(f).await;
    let elapsed_time_human_readable: humantime::Duration = elapsed_time.into();
    log::info!("{} took {}", op_description, elapsed_time_human_readable);
    res
}

/// Executes f and measures how long it took.
pub fn log_execution_time<F, T>(op_description: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let (res, elapsed_time) = measure(f);
    let elapsed_time_human_readable: humantime::Duration = elapsed_time.into();
    log::info!("{} took {}", op_description, elapsed_time_human_readable);
    res
}
