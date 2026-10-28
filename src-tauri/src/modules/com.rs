/// WinRT async operations are hot-started, so they can be waited on without an executor.
/// Commands run on Tauri's pooled threads, which have no runtime to `block_on` against,
/// hence this explicit poll-and-sleep driver with an inert waker. Returns `None` if the
/// operation outlives the deadline, which drops (releases) it rather than hanging the thread.
#[cfg(windows)]
pub fn wait<F, Fut, T>(into: F) -> Option<T>
where
    F: std::future::IntoFuture<IntoFuture = Fut, Output = T>,
    Fut: std::future::Future<Output = T> + Unpin,
{
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    use std::time::{Duration, Instant};

    unsafe fn clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &VTABLE)
    }
    unsafe fn ignore(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, ignore, ignore, ignore);

    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    let mut future = into.into_future();
    let started = Instant::now();

    loop {
        match Pin::new(&mut future).poll(&mut context) {
            Poll::Ready(value) => return Some(value),
            // A player that never answers would otherwise park this thread for good.
            Poll::Pending if started.elapsed() > Duration::from_secs(2) => return None,
            Poll::Pending => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

/// COM has to be initialised per thread, and Tauri runs each command on a pooled worker
/// thread, so the init is memoised in thread-local storage instead of done at startup.
/// MTA is required because these calls block.
#[cfg(windows)]
pub fn ensure_mta() -> Result<(), String> {
    use std::cell::Cell;

    thread_local! {
        static INITIALISED: Cell<bool> = const { Cell::new(false) };
    }

    INITIALISED.with(|done| {
        if done.get() {
            return Ok(());
        }
        let result = unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_MULTITHREADED,
            )
        };
        // Tauri's command threads are already STA-initialised, and asking for MTA there is
        // refused rather than honoured; the existing apartment serves these calls fine.
        if result.is_ok() || result == windows::Win32::Foundation::RPC_E_CHANGED_MODE {
            done.set(true);
            Ok(())
        } else {
            Err(format!("CoInitializeEx failed: {result}"))
        }
    })
}
