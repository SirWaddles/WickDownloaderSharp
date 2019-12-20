use tokio::runtime;
use tokio::time;

#[no_mangle]
pub extern fn initialize() -> *mut runtime::Runtime {
    let rt = runtime::Builder::new()
        .enable_time()
        .threaded_scheduler()
        .core_threads(4)
        .on_thread_start(|| println!("Thread Started"))
        .on_thread_stop(|| println!("Thread Stopped"))
        .build()
        .unwrap();

    println!("Starting Runtime");
    Box::into_raw(Box::new(rt))
}

#[no_mangle]
pub extern fn destroy(ptr: *mut runtime::Runtime) {
    println!("Stopping Runtime");
    if ptr.is_null() { return; }
    unsafe { Box::from_raw(ptr); }
}

#[no_mangle]
pub extern fn notify_me(rtptr: *mut runtime::Runtime, cb: extern fn(i: u32)) {
    println!("Sending Notification");
    let rt = unsafe {
        assert!(!rtptr.is_null());
        &mut *rtptr
    };

    rt.spawn(async move {
        time::delay_for(time::Duration::new(5 ,0)).await;
        cb(5);
    });
}