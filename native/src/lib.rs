use tokio::runtime;
use std::sync::{Arc, Mutex};
use wickdl::{ServiceState, PakService};
use libc::{c_char};
use std::ffi::{CStr, CString};
use std::fmt;
#[macro_use]
extern crate lazy_static;

pub struct DownloaderState {
    runtime: Arc<runtime::Runtime>,
    service: Option<Arc<ServiceState>>,
}

lazy_static! {
    static ref LAST_ERROR: Mutex<String> = Mutex::new("No Error".to_owned());
}

// https://stackoverflow.com/a/27650405/3479580
struct HexSlice<'a>(&'a [u8]);

impl<'a> HexSlice<'a> {
    fn new<T>(data: &'a T) -> HexSlice<'a> 
        where T: ?Sized + AsRef<[u8]> + 'a
    {
        HexSlice(data.as_ref())
    }
}

impl<'a> fmt::Display for HexSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
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
                    service: Some(Arc::new(service)),
                })), 0);
            },
            Err(err) => {
                set_last_error(format!("{}", err));
                cb(Box::into_raw(Box::new(DownloaderState {
                    runtime: rt2,
                    service: None,
                })), err.get_code());
            },
        };
    });
}

fn set_last_error(err: String) {
    let mut lock = LAST_ERROR.lock().unwrap();
    *lock = err;
}

#[no_mangle]
pub extern fn get_last_error() -> *mut c_char {
    let c_str = CString::new((*LAST_ERROR.lock().unwrap()).clone()).unwrap();
    c_str.into_raw()
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
        &*ptr
    };

    let service = match &state.service {
        Some(data) => data,
        None => return std::ptr::null_mut(),
    };

    Box::into_raw(Box::new(VecStringHead {
        contents: service.get_paks(),
        index: 0,
    }))
}

#[no_mangle]
pub extern fn get_pak(ptr: *mut DownloaderState, rfile: *const c_char, rkey: *const c_char, cb: extern fn(pak: *mut PakService, err: u32)) {
    let state = unsafe {
        assert!(!ptr.is_null());
        &*ptr
    };
    let file = get_string(rfile);
    let key = get_string(rkey);

    let service = match &state.service {
        Some(data) => Arc::clone(&data),
        None => {
            cb(std::ptr::null_mut(), 13);
            return;
        },
    };

    state.runtime.spawn(async move {
        match service.get_pak(file, key).await {
            Ok(pak) => {
                cb(Box::into_raw(Box::new(pak)), 0);
            },
            Err(err) => {
                set_last_error(format!("{}", err));
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

#[repr(C)]
pub struct FileDataReturn {
    pub content: *mut c_char,
    pub err: u32,
}

#[no_mangle]
pub extern fn get_file_hash(ptr: *mut PakService, rfile: *const c_char, rdata: *mut FileDataReturn) {
    let pak = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let data = unsafe {
        assert!(!rdata.is_null());
        &mut *rdata
    };
    let file = get_string(rfile);
    
    let hash = match pak.get_hash(&file) {
        Ok(data) => data,
        Err(err) => {
            set_last_error(format!("{}", err));
            data.content = std::ptr::null_mut();
            data.err = err.get_code();
            return;
        }
    };

    let hash_str = format!("{}", HexSlice::new(&hash));
    let c_str = CString::new(hash_str).unwrap();
    data.content = c_str.into_raw();
    data.err = 0;
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
                set_last_error(format!("{}", err));
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
