use crate::ecs::testing::{self};
use std::fmt::Write;
use std::fs::OpenOptions;
use std::io::Seek;
use std::net::TcpStream;
use std::time::Duration;
use std::{default, fmt};
use std::{
    sync::atomic::{self, AtomicIsize, AtomicUsize},
    thread::{self, JoinHandle},
};

#[derive(Debug, Clone)]
pub enum LogType {
    INFO,
    WARNING,
    ERROR,
    METRIC,
}

#[derive(Debug, Default, Clone)]
pub struct Log {
    pub message: String,
    pub tpe: LogType,
    pub sync: bool,
}

pub struct LogsQueue {
    pub in_progress: AtomicUsize,
    pub completed: AtomicUsize,
    pub logs: Vec<Log>,
}

enum LoggerState {
    UnInit = 0 as isize,
    InProgress = 1 as isize,
    Finishing = 2 as isize,
}

//   TODO: If we store/give each thread id form 1..63, and store info about thread somewhere in queue
//   for example in {queue.in_progress} property.
//   Then with {handle.is_finished} it is possible to check if thread still alive.
//   - So in case of thread panic we could unlock queue and free-memory.
//   But the biggest problem can be memory corruption, i.e we can't just drop vector with item
//   that was corrupted, to do so we must know which item was corrupted and forget it.
//   - So the simpler solution will be, 1 replace string with Enum with `Invalid` state
//   and 2 after writing buffer, replace it with finished String. In such case we are safe against
//   out-of-memory problem and vector memory modification will be done with operation that couldn't fail
pub static mut LOGGER: Option<JoinHandle<()>> = None;
pub static mut QUEUES: Vec<LogsQueue> = Vec::new();
pub static QUEUE_ID: AtomicUsize = AtomicUsize::new(0);
pub static LOGGER_STATE: AtomicIsize = AtomicIsize::new(LoggerState::UnInit as isize);
pub const LOGS_BUFFER_SIZE: usize = 1024;
pub const MAX_LOGS_PER_FILE: usize = 15_000;

pub static mut DEBUG_SLEEP_NS: u64 = 0;

testing::mock!(true, LoggerMock {
    sleep: fn() -> (),
    sender: ::std::sync::mpsc::Sender<String>
});

testing::mock!(LogMock {
    sleep: fn() -> ()
});

pub fn sleep(ns: u64) {
    #[cfg(miri)]
    std::thread::sleep(Duration::from_nanos(1));

    #[cfg(not(miri))]
    {
        let start_time =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        while std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
            - start_time
            < ns as u128
        {}
    }
}

pub fn init_logger(path: &str) {
    let path = String::from(path);
    testing::mock_setup!(LoggerMock, true);
    if LOGGER_STATE.load(atomic::Ordering::Acquire) != LoggerState::UnInit as isize {
        panic!("Logger# finish previous logger first")
    }

    #[allow(unused_variables, unused_mut)]
    let mut devtools: Option<TcpStream> = None;
    #[cfg(feature = "release")]
    let mut devtools = {
        let stream = unsafe {
            crate::ecs::devtools::server::DEVTOOLS.as_mut().map(|stream| stream.try_clone())
        };
        match stream {
            Some(Ok(stream)) => Some(stream),
            _ => {
                // println!("Logging# can't connect to devtools");
                None
            }
        }
    };

    let mut queue1 = Vec::new();
    let mut queue2 = Vec::new();
    queue1.resize(LOGS_BUFFER_SIZE + 1, Log::default());
    queue2.resize(LOGS_BUFFER_SIZE + 1, Log::default());

    unsafe {
        QUEUES = vec![
            LogsQueue {
                in_progress: AtomicUsize::new(0),
                completed: AtomicUsize::new(0),
                logs: queue1,
            },
            LogsQueue {
                in_progress: AtomicUsize::new(0),
                completed: AtomicUsize::new(0),
                logs: queue2,
            },
        ];
    };

    LOGGER_STATE.store(LoggerState::InProgress as isize, atomic::Ordering::Release);

    let logger: JoinHandle<()> = thread::spawn(move || {
        use std::io::Write;
        // if logs impossible to create log only to stdout
        #[cfg(not(miri))]
        let start_time = libc_strftime::epoch();

        let path1 = format!("{path}_1.txt");
        let mut logs_store1 = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path1)
            .unwrap_or_else(|_| panic!("Logs can't be initiated for file {}", path1));

        let path2 = format!("{path}_2.txt");
        let mut logs_store2 = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path2)
            .unwrap_or_else(|_| panic!("Logs can't be initiated for file {}", path2));

        let mut total_processed_logs = 0;
        let mut buffer = String::new();
        'queue_loop: loop {
            let queue_id = QUEUE_ID.load(std::sync::atomic::Ordering::Acquire);
            let queue = unsafe { &QUEUES[queue_id] };

            if queue.in_progress.load(std::sync::atomic::Ordering::Acquire) != 0 {
                mock_value!((), |ctx: &mut LoggerMock| { (ctx.sleep)() });

                QUEUE_ID
                    .swap(if queue_id == 0 { 1 } else { 0 }, std::sync::atomic::Ordering::AcqRel);

                let mut last_processed_log = 0;

                'memory_guard: loop {
                    let completed_len = queue.completed.load(std::sync::atomic::Ordering::Acquire);
                    mock_value!((), |ctx: &mut LoggerMock| { (ctx.sleep)() });

                    if completed_len != queue.in_progress.load(std::sync::atomic::Ordering::Acquire)
                    {
                        // we can't access safely queue content because other thread could still use it
                        continue 'memory_guard;
                    }

                    #[cfg(not(miri))]
                    let current_time = libc_strftime::strftime_gmt("%d-%m-%Y %H:%S", start_time);
                    let current_time = "00.00.0000 00:00";

                    buffer.truncate(0);
                    for idx in last_processed_log..completed_len {
                        let _ = write!(
                            buffer,
                            "{} ⚝ {current_time} ⚝ {}\n",
                            &queue.logs[idx].tpe, &queue.logs[idx].message
                        );

                        mock_value!(
                            {
                                let _ = match devtools {
                                    Some(ref mut devtools) => {
                                        write!(
                                            devtools,
                                            "{} ⚝ {current_time} ⚝ {}\n",
                                            &queue.logs[idx].tpe, &queue.logs[idx].message
                                        )
                                    }
                                    _ => Ok(()),
                                };
                            },
                            |ctx: &mut LoggerMock| {
                                let _ = ctx.sender.send(queue.logs[idx].message.clone());
                                ()
                            }
                        );

                        let next_cycle = (total_processed_logs + 1) / MAX_LOGS_PER_FILE;
                        let curr_cycle = total_processed_logs / MAX_LOGS_PER_FILE;
                        let is_next_store = next_cycle > curr_cycle;
                        if is_next_store {
                            if next_cycle % 2 == 0 {
                                let _ = logs_store1.set_len(0);
                                let _ = logs_store1.rewind();
                                let _ = write!(logs_store2, "{}", buffer);
                                buffer.truncate(0);
                            } else {
                                let _ = logs_store2.set_len(0);
                                let _ = logs_store2.rewind();
                                let _ = write!(logs_store1, "{}", buffer);
                                buffer.truncate(0);
                            }
                        }

                        total_processed_logs += 1;
                    }

                    if (total_processed_logs / MAX_LOGS_PER_FILE) % 2 == 0 {
                        let _ = write!(logs_store1, "{}", buffer);
                    } else {
                        let _ = write!(logs_store2, "{}", buffer);
                    }

                    mock_value!((), |ctx: &mut LoggerMock| (ctx.sleep)());

                    last_processed_log = completed_len;

                    let in_progress_swap = queue.in_progress.compare_exchange(
                        completed_len,
                        0,
                        std::sync::atomic::Ordering::AcqRel,
                        std::sync::atomic::Ordering::Acquire,
                    );

                    if in_progress_swap.is_err() {
                        // someone modified queue while we processing logs, repeat
                        continue 'memory_guard;
                    }

                    mock_value!((), |ctx: &mut LoggerMock| { (ctx.sleep)() });

                    let completed_swap = queue.completed.compare_exchange(
                        completed_len,
                        0,
                        std::sync::atomic::Ordering::AcqRel,
                        std::sync::atomic::Ordering::Acquire,
                    );

                    if completed_swap.is_err() {
                        // someone modified queue while we processing logs, rollback previous operation and repeat
                        queue
                            .in_progress
                            .fetch_add(completed_len, std::sync::atomic::Ordering::AcqRel);

                        mock_value!((), |ctx: &mut LoggerMock| { (ctx.sleep)() });

                        continue 'memory_guard;
                    }

                    #[cfg(test)]
                    {
                        println!(
                            "queue# {}, completed_size: {} - {} - {}, total: {}",
                            queue_id,
                            completed_len,
                            queue.completed.load(std::sync::atomic::Ordering::Acquire),
                            queue.in_progress.load(std::sync::atomic::Ordering::Acquire),
                            total_processed_logs
                        );
                    }

                    break 'memory_guard;
                }

                continue 'queue_loop;
            }

            if LOGGER_STATE.load(atomic::Ordering::Acquire) == LoggerState::Finishing as isize {
                LOGGER_STATE.store(LoggerState::UnInit as isize, atomic::Ordering::Release);
                break 'queue_loop;
            } else {
                std::thread::park_timeout(std::time::Duration::from_millis(100));
            }
        }
    });

    unsafe { LOGGER.replace(logger) };
}

impl fmt::Display for LogType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LogType::INFO => "INFO  ",
                LogType::WARNING => "WARN  ",
                LogType::ERROR => "ERROR ",
                LogType::METRIC => "METRIC",
            }
        )
    }
}

impl default::Default for LogType {
    fn default() -> Self {
        LogType::INFO
    }
}

// Safety guaranties
//
// * Memory ordering:
//   AcqRel/Acquire/Release should be enough, because both {QUEUE_ID} and
//   {queue.in_progress/completed} is counter like values.
//   so we don't need additional ordering guaranties from SeqCst
//
// * Pinning:
//   {queue.logs} must never re-allocate it's content, this
//   was accomplished by resizing initial vec for expected size
//   and never calling methods that could increase/reduce vector
//   capacity
//
// * Logs overflow:
//   {queue.completed} must never be higher than {LOGS_BUFFER_SIZE},
//   this was accomplished by firstly increasing {queue.in_progress} and
//   checking current queue logs size, in case of overflowing,
//   we rollback {queue.in_progress} and repeat log process with another queue
//
// * SAFETY & References exclusivity:
//   - {queue} Must be always referenced as immutable
//   - {queue.log[idx]} Could be referenced both mutable & immutable.
//     To prevents rust ref rule violation, only non logger threads
//     could have exclusive mutable reference to one of remains
//     log item.
//     And only logger thread can immutable reference to [0..[first mutable log slot])
//     log item.
//     Check Data Races section for more info about implementation
//
// * Data races:
//   - In general data races prevented by granularly restricting access to vector memory in multiple threads.
//   i.e only one thread could edit/process any vector item at time. This implementation
//   provide such guaranty through {queue.completed_size}. i.e SINGLE processing thread
//   could immutable use 0..{queue.completed_size} items, and MULTIPLE logging threads could
//   use only one mutable {queue.completed + 1} item. {queue.completed + 1} - is atomic
//   operation, so if first thread take item = 1, then second thread will take
//   item = 2 etc...
//   - Additional guaranies is provided by {queue.in_progress}, knowing length of {queue.in_progress}, allow
//   us to safely check all if logging tread finished logging, and reset queue state only when no-one
//   use query
//
// * Performances & Memory: ~3MB [1024], x5 faster than println!()
//   - In comparison with println!(), if LOGGER could process logs faster than they come into it
//     then it should be faster at least x5 times, otherwise at least x2 times
//   - Double buffering reduces wait time if queue was fully filled with logs
//   - LOGGER use double buffering with buffer size === 1024, items inside buffer will be reused
//     so all logging after first don't need to allocate memory, we also never drop or shrink vector items.
//     With average length of log 120, total memory usage by logger will be around 3MB
//   - LOGGER will automatically went into sleep mode if there nothing to log, and first log
//     should wake it up
//   - Processing single log always require 3 atomic operation + one random access to random memory
//     + writing log inside string
#[macro_export]
macro_rules! _log {
  ($log_type:expr, { is_sync: $is_sync:expr, is_dev: $is_dev:ident }, $($args:tt)*) => {
      'overflow_guard: loop {
          use std::fmt::Write;
          use $crate::ecs::logger::LogType;
          $crate::ecs::testing::mock_setup!(LogMock, $is_dev);

          #[cfg(test)]
          $crate::cond!(not $is_dev, {
            // use stdout inside testing environment
            println!($($args)*);
            break 'overflow_guard;
          });

          let log_type: LogType = $log_type;
          let queue_id = $crate::ecs::logger::QUEUE_ID.load(std::sync::atomic::Ordering::Acquire);

          let log_idx = unsafe { &$crate::ecs::logger::QUEUES[queue_id] }.in_progress.fetch_add(1, std::sync::atomic::Ordering::AcqRel);

          $crate::cond!($is_dev, {
            mock_value!((), |ctx: &mut LogMock| { (ctx.sleep)() });
          });

          if log_idx >= $crate::ecs::logger::LOGS_BUFFER_SIZE {
              unsafe { &$crate::ecs::logger::QUEUES[queue_id] }.in_progress.fetch_sub(1, std::sync::atomic::Ordering::Release);
              continue 'overflow_guard;
          }

          let log = unsafe { &mut $crate::ecs::logger::QUEUES[queue_id].logs[log_idx] };
          log.message.truncate(0);
          log.tpe = log_type;
          log.sync = $is_sync;
          let _ = write!(log.message, $($args)*);
          let sync_log = log.sync;
          #[allow(dropping_references)]
          drop(log);

          $crate::cond!($is_dev, {
            mock_value!((), |ctx: &mut LogMock| { (ctx.sleep)() });
          });

          let id_in_queue = unsafe { &$crate::ecs::logger::QUEUES[queue_id] }.completed.fetch_add(1, std::sync::atomic::Ordering::AcqRel);

          if log_idx == 0 {
              match unsafe { &$crate::ecs::logger::LOGGER } {
                  Some(ref handle) => ::std::thread::Thread::unpark(handle.thread()),
                  None => (),
              };
          }

          if sync_log {
              while unsafe { &$crate::ecs::logger::QUEUES[queue_id] }.completed.load(::std::sync::atomic::Ordering::Acquire) >= id_in_queue + 1 {
              }
          }

          break 'overflow_guard;
      }
  };
}

#[allow(unused)]
pub(crate) use _log;

#[macro_export]
macro_rules! log {
    ($log_type:tt, $($args:tt)*) => {
        $crate::_log!(LogType::$log_type, { is_sync: false, is_dev: false }, $($args)*);
    };
}

#[allow(unused)]
pub(crate) use log;

#[macro_export]
macro_rules! log_sync {
    ($log_type:tt, $($args:tt)*) => {
        $crate::ecs::_log!(LogType::$log_type, { is_sync: true, is_dev: false }, $($args)*);
    };
}

#[allow(unused)]
pub(crate) use log_sync;

// ==========
// = TESTS  =
// ==========
#[test]
fn test_write() {
    use std::fmt::Write;

    let mut string = String::default();
    write!(&mut string, "Hello world 1").expect("Write into message");
    write!(&mut string, "Hello world 2").expect("Write into message");

    assert_eq!(string, "Hello world 1Hello world 2");
    string.truncate(0);
    write!(&mut string, "Hello World 2").expect("Write into message");
    assert_eq!(string, "Hello World 2");
}

#[test]
fn logger_in_other_tests() {
    log!(INFO, "Hello");
}

#[test]
#[cfg(not(miri))]
fn logger_file_storage() {
    logger_file_storage_base("./logs/logs").unwrap();
}

pub fn logger_file_storage_base(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;

    macro_rules! log_dev {
        ($log_type:tt, $($args:tt)*) => {
            $crate::_log!(LogType::$log_type, { is_sync: false, is_dev: true }, $($args)*);
        };
    }

    macro_rules! log_dev_sync {
        ($log_type:tt, $($args:tt)*) => {
            $crate::_log!(LogType::$log_type, { is_sync: true, is_dev: true }, $($args)*);
        };
    }

    crate::ecs::logger::init_logger(path);

    for i in 0..9 {
        log_dev!(INFO, "{}", i);
    }
    log_dev_sync!(INFO, "{}", 10);

    let mut file1 = std::fs::File::open(format!("{path}_1.txt"))?;
    let mut contents1 = String::new();
    file1.read_to_string(&mut contents1)?;

    assert_eq!(contents1.lines().count(), 10);

    for i in 0..MAX_LOGS_PER_FILE {
        log_dev!(INFO, "{}", i);
    }
    log_dev_sync!(INFO, "{}", MAX_LOGS_PER_FILE + 1);

    let mut file2 = std::fs::File::open(format!("{path}_2.txt"))?;
    let mut contents2 = String::new();
    file2.read_to_string(&mut contents2)?;

    assert_eq!(contents2.lines().count(), 11);
    LOGGER_STATE.store(LoggerState::Finishing as isize, atomic::Ordering::Release);
    let _ =
        unsafe { LOGGER.take() }.ok_or("Logger# logger handle must be exclusive accesible")?.join();

    Ok(())
}

#[test]
#[cfg(not(miri))]
fn test_logger_all() {
    use std::sync::mpsc::channel;

    test_logger(100, sleep_0);
    test_logger(10000, sleep_0);
    test_logger(100, sleep_100);
    test_logger(10000, sleep_100);

    // == HELPERS ==
    // =============

    macro_rules! log_dev {
        ($log_type:tt, $($args:tt)*) => {{
            $crate::_log!(LogType::$log_type, { is_sync: false, is_dev: true }, $($args)*);
        }};
    }

    fn test_logger(logs_amount: usize, sleep: fn() -> ()) {
        let (sender, receiver) = channel::<String>();

        testing::mock_run!(super::init_logger("logs/test_logger_all"), |mock: &mut LoggerMock| {
            mock.sender.replace(sender.clone());
            mock.sleep.replace(sleep);
        });

        let logs_size = logs_amount;
        let thread_amount = 2;
        let total_logs = logs_size * thread_amount;

        let t1 = thread::spawn(move || {
            println!("start: info");
            for i in 0..logs_size {
                testing::mock_run!(log_dev!(INFO, "{}", i), |mock: &mut LogMock| {
                    mock.sleep.replace(sleep);
                });
                if i % 100 == 0 {
                    println!("log: {logs_size} < {i}");
                }
            }
            println!("end: info");
        });

        let t2 = thread::spawn(move || {
            println!("start: error");
            for i in logs_size..logs_size * thread_amount {
                testing::mock_run!(log_dev!(ERROR, "{}", i), |mock: &mut LogMock| {
                    mock.sleep.replace(sleep);
                });
                if i % 1000 == 0 {
                    println!("error: {i}");
                }
            }
            println!("end: error");
        });

        let expected_logs: Vec<u64> = (0..total_logs as u64).collect();
        let mut logs: Vec<u64> = Vec::new();
        for _ in 0..total_logs {
            logs.push(receiver.recv().unwrap().parse().unwrap());
        }

        logs.sort();
        assert_eq!(logs.len(), total_logs);
        assert_eq!(logs, expected_logs);

        let _ = t1.join();
        let _ = t2.join();

        LOGGER_STATE.store(LoggerState::Finishing as isize, atomic::Ordering::Release);
        let _ = unsafe { LOGGER.take() }
            .expect("Logger# logger handle must be exclusive accesible")
            .join();
    }

    fn sleep_0() {
        sleep(0);
    }

    fn sleep_100() {
        sleep(10000);
    }
}
