#[macro_export]
macro_rules! perf {
    ($start:expr, $end:expr, $fn_name:expr) => {
        println!("PERF: {} - {}", $fn_name, ($end - $start).as_secs_f64())
    };
}
