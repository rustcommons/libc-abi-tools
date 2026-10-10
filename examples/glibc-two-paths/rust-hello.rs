use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

static DROPS: AtomicUsize = AtomicUsize::new(0);
struct Probe;
impl Drop for Probe {
    fn drop(&mut self) { DROPS.fetch_add(1, Ordering::SeqCst); }
}
thread_local! {
    static COUNTER: Cell<u32> = const { Cell::new(7) };
    static PROBE: Probe = const { Probe };
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    assert!(std::panic::catch_unwind(|| panic!("intentional unwind test")).is_err());
    COUNTER.with(|counter| counter.set(11));
    let result = std::thread::spawn(|| {
        PROBE.with(|_| {});
        COUNTER.with(|counter| {
            assert_eq!(counter.get(), 7);
            counter.set(23);
        });
        let values = vec![1u64, 2, 3, 4];
        values.into_iter().sum::<u64>()
    }).join().expect("thread failed");
    assert_eq!(result, 10);
    assert_eq!(DROPS.load(Ordering::SeqCst), 1);
    COUNTER.with(|counter| assert_eq!(counter.get(), 11));
    println!("Rust allocation, thread/TLS, and unwinding work!");
}
