use tokio::runtime;
use std::sync::Arc;
use wickdl::{ServiceState, PakService};
use libc::{c_char};
use std::ffi::{CStr, CString};

pub struct DownloaderState {
    runtime: Arc<runtime::Runtime>,
    service: Arc<ServiceState>,
}

#[no_mangle]
pub extern fn initialize(cb: extern fn(state: *mut DownloaderState, err: u32)) {
    let rt = Arc::new(runtime::Builder::new()
        .enable_all()
        .threaded_scheduler()
        .core_threads(4)
        .build()
        .unwrap());

    let rt2 = Arc::clone(&rt);
    rt.spawn(async move {
        match ServiceState::new().await {
            Ok(service) => {
                cb(Box::into_raw(Box::new(DownloaderState {
                    runtime: rt2,
                    service: Arc::new(service),
                })), 0);
            },
            Err(err) => {
                cb(std::ptr::null_mut(), err.get_code());
            },
        };
    });
}

fn get_string(s: *const c_char) -> String {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };

    c_str.to_str().unwrap().to_string()
}

#[no_mangle]
pub extern fn get_pak_names(ptr: *mut DownloaderState) -> *mut VecStringHead {
    let state = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    Box::into_raw(Box::new(VecStringHead {
        contents: state.service.get_paks(),
        index: 0,
    }))
}

#[no_mangle]
pub extern fn get_pak(ptr: *mut DownloaderState, rfile: *const c_char, rkey: *const c_char, cb: extern fn(pak: *mut PakService, err: u32)) {
    let state = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    let file = get_string(rfile);
    let key = get_string(rkey);

    let service = Arc::clone(&state.service);

    state.runtime.spawn(async move {
        match service.get_pak(file, key).await {
            Ok(pak) => {
                cb(Box::into_raw(Box::new(pak)), 0);
            },
            Err(err) => {
                cb(std::ptr::null_mut(), err.get_code());
            },
        };
    });
}

#[no_mangle]
pub extern fn get_pak_mount(ptr: *mut PakService) -> *mut c_char {
    let pak = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let c_str = CString::new(pak.get_mount_point()).unwrap();
    c_str.into_raw()
}

#[no_mangle]
pub extern fn get_file_names(ptr: *mut PakService) -> *mut VecStringHead {
    let pak = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    Box::into_raw(Box::new(VecStringHead {
        contents: pak.get_files(),
        index: 0,
    }))
}

#[no_mangle]
pub extern fn get_file_data(rtptr: *mut DownloaderState, pakptr: *mut PakService, rfile: *const c_char, cb: extern fn (data: *mut u8, length: u32, err: u32)) {
    let state = unsafe {
        assert!(!rtptr.is_null());
        &mut *rtptr
    };
    let pak = unsafe {
        assert!(!pakptr.is_null());
        &mut *pakptr
    };
    let file = get_string(rfile);

    state.runtime.spawn(async move {
        match pak.get_data(&file).await {
            Ok(mut data) => {
                cb(data.as_mut_ptr(), data.len() as u32, 0);
            },
            Err(err) => {
                cb(std::ptr::null_mut(), 0, err.get_code());
            },
        };
    });
}

#[no_mangle]
pub extern fn destroy(ptr: *mut DownloaderState) {
    if ptr.is_null() { return; }
    unsafe { Box::from_raw(ptr); }
}

pub struct VecStringHead {
    contents: Vec<String>,
    index: usize,
}

#[no_mangle]
pub extern fn vec_string_get_next(ptr: *mut VecStringHead) -> *mut c_char {
    let container = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let string = match container.contents.get(container.index) {
        Some(data) => data,
        None => return std::ptr::null_mut(),
    };

    container.index += 1;
    let c_str = CString::new(string.to_owned()).unwrap();
    c_str.into_raw()
}

#[no_mangle]
pub extern fn free_pak(ptr: *mut PakService) {
    if ptr.is_null() { return; }
    unsafe { Box::from_raw(ptr); }
}

#[no_mangle]
pub extern fn free_vec_string(ptr: *mut VecStringHead) {
    if ptr.is_null() { return; }
    unsafe { Box::from_raw(ptr); }
}

#[no_mangle]
pub extern fn free_string(ptr: *mut c_char) {
    if ptr.is_null() { return; }
    unsafe { CString::from_raw(ptr); }
}
