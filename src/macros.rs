#[macro_export]
macro_rules! perf {
    ($start:expr, $fn_name:expr) => {
        let end = Instant::now();
        println!("PERF: {} - {}", $fn_name, (end - $start).as_secs_f64())
    };
}
