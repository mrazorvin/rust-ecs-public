// a i32 value that determine duration in milliseconds since last cycle
// first 3 bits is cycle bites and could be 0..7
// other 29 bits is milliseconds since last cycle

// every metric could log itself since exacly amount of milliseconds
// or when cycle changed, for exmaple we could log every 1000milliseconds
// but if cycle changed from 0 -> 1, we also must log, since we don't
// know how many seconds was passed since last cycle changes
// if we faced last lycle, then reset cycle counter to 0

// the basic loggin mehanismus consist of two steps
// first one is increaseing atomic metric by 1
// then analyze returning value
// 1. Analyzing metric part
// 1.1 If next incresing metric could overflow into timer part
// 1.2 Then we need switch last bit into overflow mode, reset metric
//     part to 0 and log info that metric owerflow
//     we also need be sure that metric in oweflow mode never overflow again
// 2.  Analyzing timer part
// 2.1 If milliseconds part empty, then log metric directly
// 2.2 If diffrenece between current cycle and metrcis not that hight, then
//     do nothing
// 2.3 If diffrenece betwen current cycle enought, then log metric
// 2.4 If cycle diffrent or milliseconds lesser than stored one, then log metric
// 3.2 If loging happened, we need to write time when it happened (and possible reset metric if metric in cycle mode)

// how many operations needed to metrics check in two cases
// 1: Both ID & TIME are stored in separate variable (valid for global variables and metric variable)
// 2. ID & TIME are store in separete variables

// it's always faster to store single Atomic64UInt for conunter, because of random memory access i.e on client side we must store this variables
// it's look like it possible to optimize bit and comparion operation for since time is linear and ussual difference between time is somewhere araound
// 16ms, but increasing cylce id lead us to huge number jump so after extracting timeinfo from UInt one comparion may be enought ????

pub static mut METRICS_CYCLE: u32 = 0;
pub static mut TIMESTAPM: u128 = 0;

const FULL_CYCLE_BITS_LEN: u32 = 32;
const CYCLE_ID_BITS_LEN: u32 = 3;
const CYCLE_TIME_MASK: u32 = u32::MAX >> CYCLE_ID_BITS_LEN;

pub const METRICS_LOG_INTERVAL_MS: u32 = 1000;
pub const METRICS_OVERFLOW_BUFFER: u32 = 10000;

testing::ctx!(
    $,
    cycle,
    Context {
        time_since_prev_cycle: std::time::Duration
    }
);

pub fn next_cycle() {
    let prev_time = unsafe { TIMESTAPM };
    let next_time = cycle_ctx!(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap(),
        |ctx: &mut Context| { *ctx.time_since_prev_cycle }
    );

    let next_time = next_time.as_millis();
    let invalid_time = next_time <= prev_time;
    if invalid_time {
        return;
    }

    let time_init = prev_time != 0;
    if time_init {
        let time_diff = (next_time - prev_time) as u32;
        let metrics_cycle_ref = unsafe { &mut METRICS_CYCLE };

        let mut cycle_time = *metrics_cycle_ref & CYCLE_TIME_MASK;
        let mut cycle_id = *metrics_cycle_ref >> (FULL_CYCLE_BITS_LEN - CYCLE_ID_BITS_LEN);

        cycle_time = cycle_time.saturating_add(time_diff);

        let time_overflow = cycle_time > CYCLE_TIME_MASK;
        if time_overflow {
            cycle_time = 0;
            cycle_id = (cycle_id + 1) % 8;
        }

        cycle_id = if cycle_id == 0 { 1 } else { cycle_id };

        *metrics_cycle_ref = cycle_time | (cycle_id << (FULL_CYCLE_BITS_LEN - CYCLE_ID_BITS_LEN));
    }

    unsafe { TIMESTAPM = next_time };
}

#[macro_export]
macro_rules! metric {
    ($($pattern:tt)*) => {{
        use ::std::sync::atomic::{AtomicU64, Ordering};

        let overflow_buffer = $crate::ecs::logger::metrics::METRICS_OVERFLOW_BUFFER;
        let metrics_log_interval_ms = $crate::ecs::logger::metrics::METRICS_LOG_INTERVAL_MS;
        let metrics_cycle = unsafe { $crate::ecs::logger::metrics::METRICS_CYCLE };

        static METRIC: AtomicU64 = AtomicU64::new($crate::ecs::logger::metrics::METRICS_OVERFLOW_BUFFER as u64);
        let metric = METRIC.fetch_add(1, Ordering::Release);
        let (cycle, counter) = ((metric >> 32) as u32, metric as u32);
        let (time_diff, cycle_overflow) = metrics_cycle.overflowing_sub(cycle);
        let next_counter = counter.wrapping_add(1);
        let counter_overflow = next_counter < overflow_buffer; // overflow detection

        if time_diff >= metrics_log_interval_ms || cycle_overflow || counter == overflow_buffer {
            $crate::log!(METRIC, $($pattern)*, counter = next_counter);
            let new_metric = ((metrics_cycle as u64) << 32) | next_counter as u64;
            let _ = METRIC.compare_exchange(metric + 1, new_metric, Ordering::AcqRel, Ordering::Relaxed);
        } else if counter_overflow  {
            METRIC.fetch_sub(1, Ordering::Relaxed);
        }

        &METRIC
    }};
}

#[allow(unused)]
pub(crate) use metric;

use crate::ecs::testing;

#[test]
fn macro_test_metric() {
    use std::sync::atomic::{AtomicU64, Ordering::SeqCst};

    unsafe { METRICS_CYCLE = 0 };

    fn metric() -> &'static AtomicU64 {
        static METRIC_PLACEHOLDER: AtomicU64 = AtomicU64::new(0);
        return metric!("test-metric# {counter}");
    }

    let overflow_buffer = METRICS_OVERFLOW_BUFFER as u64;

    assert_eq!(metric().load(SeqCst), 1 + overflow_buffer);

    unsafe { METRICS_CYCLE = u32::MAX };

    assert_eq!(metric().load(SeqCst), ((u32::MAX as u64) << 32) | 2 + overflow_buffer);

    unsafe { METRICS_CYCLE = 0 };

    assert_eq!(metric().load(SeqCst), 3 + overflow_buffer);

    metric().store(u32::MAX as u64, SeqCst);

    assert_eq!(metric().load(SeqCst), u32::MAX as u64);

    unsafe { METRICS_CYCLE = u32::MAX };
    metric().store(u32::MAX as u64, SeqCst);
    assert_eq!(metric().load(SeqCst), ((u32::MAX as u64) << 32) | 0);
}

#[test]
fn test_next_cycle_and_metric() {
    use std::time::Duration;
    let ctx = cycle_guard();

    unsafe { METRICS_CYCLE = 0 };

    const METRICS_UNINIT: u32 = 0;

    let time_since_app_init_cycle1 = 1;
    let time_since_app_init_cycle2 = 10;
    let time_since_app_init_cycle3 = 15;
    let time_since_app_init_cycle4 = 20;

    // ## CYCLE 1
    ctx.init(&|ctx| {
        ctx.time_since_prev_cycle.replace(Duration::from_millis(time_since_app_init_cycle1));
    });
    self::next_cycle();
    assert_eq!(unsafe { METRICS_CYCLE }, METRICS_UNINIT);
    assert_eq!(unsafe { TIMESTAPM }, time_since_app_init_cycle1 as u128);

    // ## CYCLE 2
    ctx.init(&|ctx| {
        ctx.time_since_prev_cycle.replace(Duration::from_millis(time_since_app_init_cycle2));
    });
    self::next_cycle();
    assert_eq!(unsafe { METRICS_CYCLE }, {
        let time_since_prev_cycle =
            (time_since_app_init_cycle2 - time_since_app_init_cycle1) as u32;
        let cycle_id = 1 << (FULL_CYCLE_BITS_LEN - CYCLE_ID_BITS_LEN);

        time_since_prev_cycle | cycle_id
    });
    assert_eq!(unsafe { TIMESTAPM }, time_since_app_init_cycle2 as u128);

    // ## CYCLE 3
    ctx.init(&|ctx| unsafe {
        ctx.time_since_prev_cycle.replace(Duration::from_millis(time_since_app_init_cycle3));

        // force cyles time overflow
        METRICS_CYCLE =
            (u32::MAX >> CYCLE_ID_BITS_LEN) | (1 << (FULL_CYCLE_BITS_LEN - CYCLE_ID_BITS_LEN));
    });
    self::next_cycle();
    assert_eq!(unsafe { METRICS_CYCLE }, 0 | (2 << (FULL_CYCLE_BITS_LEN - CYCLE_ID_BITS_LEN)));
    assert_eq!(unsafe { TIMESTAPM }, time_since_app_init_cycle3 as u128);

    // ## CYCLE 4
    ctx.init(&|ctx| unsafe {
        ctx.time_since_prev_cycle.replace(Duration::from_millis(time_since_app_init_cycle4));

        // force cyles id & time overflow
        METRICS_CYCLE = u32::MAX;
    });

    self::next_cycle();
    assert_eq!(unsafe { METRICS_CYCLE }, 0 | (1 << (FULL_CYCLE_BITS_LEN - CYCLE_ID_BITS_LEN)));
    assert_eq!(unsafe { TIMESTAPM }, time_since_app_init_cycle4 as u128);
}

#[test]
fn test_next_cycle_wrapping() {
    assert_eq!(u32::MAX, u32::MAX.saturating_add(1));
    assert_eq!((u32::MAX, true), u32::MIN.overflowing_sub(1));
}
