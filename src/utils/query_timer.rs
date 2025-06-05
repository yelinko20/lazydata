use std::time::{Duration, Instant};

/// Executes a future (usually a database query) and measures how long it takes.
/// Returns a tuple of the result and the elapsed duration.
///
/// # Example
/// ```
/// let (result, duration) = query_timer(query("SELECT * FROM users").execute(&pool)).await;
/// println!("Query executed in {:?}", duration);
/// ```
///
pub async fn query_timer<F, T>(f: F) -> (T, Duration)
where
    F: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f.await;
    let elapsed = start.elapsed();
    (result, elapsed)
}
