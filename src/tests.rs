use crate::*;
use std::sync::atomic::AtomicBool;
use std::sync::RwLock;

#[test]
fn cow_faster_than_rwlock_nosleep() {
    let reader_sleep = Some(std::time::Duration::from_millis(5));
    let writer_sleep = Some(std::time::Duration::from_millis(10));
    let (time_cow, read_count_cow) =
        write_and_read_alot(true, 10, 100, reader_sleep, writer_sleep);
    let (time_rwlock, read_count_rwlock) =
        write_and_read_alot(false, 10, 100, reader_sleep, writer_sleep);

    assert!(
        time_cow < time_rwlock,
        "SyncCow should be faster than RwLock for writing and reading alot"
    );

    let cow_read_per_sec = read_count_cow as f64 / time_cow.as_secs_f64();
    let rwlock_read_per_sec = read_count_rwlock as f64 / time_rwlock.as_secs_f64();
    assert!(
        cow_read_per_sec > rwlock_read_per_sec,
        "SyncCow should enable more reads per second than RwLock"
    );
}

fn write_and_read_alot(
    use_cow: bool,
    reader_count: usize,
    write_count: usize,
    reader_sleep: Option<std::time::Duration>,
    writer_sleep: Option<std::time::Duration>,
) -> (std::time::Duration, usize) {
    let start_nr = 5;
    let cow = Arc::new(SyncCow::new(start_nr));
    let rwlock = Arc::new(RwLock::new(start_nr));
    let stopped = Arc::new(AtomicBool::new(false));
    let mut readers: std::vec::Vec<std::thread::JoinHandle<()>> = vec![];
    let global_counter = Arc::new(Mutex::new(0));
    let start = std::time::Instant::now();
    let all_reads_sequential = Arc::new(Mutex::new(true));
    for _ in 0..reader_count {
        let read_cow_clone = cow.clone();
        let stopped_clone = stopped.clone();
        let rwlock_clone = rwlock.clone();
        let global_counter = global_counter.clone();
        let all_reads_sequential = all_reads_sequential.clone();
        readers.push(std::thread::spawn(move || {
            let mut counter = 0;
            let mut read_count = 0;
            let cow_ref = read_cow_clone.as_ref();
            let rwlock_ref = rwlock_clone.as_ref();
            let mut last_read = start_nr;
            loop {
                if use_cow {
                    let read = *cow_ref.read().as_ref();
                    if read != last_read && read != last_read + 1 {
                        println!("Got {} last, but {} then", last_read, read);
                        let mut all_reads_sequential = all_reads_sequential.lock().unwrap();
                        *all_reads_sequential = false;
                    }
                    last_read = read;
                    counter += *cow_ref.read().as_ref() % 97;
                } else {
                    counter += *rwlock_ref.read().unwrap() % 97;
                }
                read_count += 1;
                if stopped_clone.load(Relaxed) {
                    assert!(
                        last_read == write_count,
                        "Read did not read the last value of write"
                    );
                    println!("{}", counter);
                    *global_counter.lock().unwrap() += read_count;
                    return;
                }
                match reader_sleep {
                    Some(time) => std::thread::sleep(time),
                    None => std::thread::yield_now(),
                }
            }
        }));
    }

    let cow_clone = cow.clone();
    let rwlock_clone = rwlock.clone();
    let write = std::thread::spawn(move || {
        let cow_ref = cow_clone.as_ref();
        let rwlock_ref = rwlock_clone.as_ref();
        if use_cow {
            assert!(*cow_ref.read().as_ref() == 5, "SyncCow has unexpected value");
        } else {
            assert!(*rwlock_ref.read().unwrap() == 5, "SyncCow has unexpected value");
        }
        loop {
            let mut val = 0;
            if use_cow {
                cow_ref.edit(|x| {
                    match writer_sleep {
                        Some(time) => std::thread::sleep(time),
                        None => std::thread::yield_now(),
                    }
                    *x += 1;
                    val = *x;
                });
            } else {
                let mut lck = rwlock_ref.write().unwrap();
                match writer_sleep {
                    Some(time) => std::thread::sleep(time),
                    None => std::thread::yield_now(),
                }
                *lck += 1;
                val = *lck;
            }
            if val >= write_count {
                return;
            }
        }
    });

    let _ = write.join();
    stopped.store(true, Relaxed);
    let time = start.elapsed();
    for join_handle in readers {
        let _ = join_handle.join();
    }
    if use_cow {
        assert!(
            *all_reads_sequential.lock().unwrap(),
            "The readers did not catch all writes sequentially, probably they were too slow"
        );
    }
    let read_count = *global_counter.lock().unwrap();
    (time, read_count)
}
