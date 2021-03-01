use tokio::runtime;
use std::sync::{Arc, Mutex};
use wickdl::{ServiceState, UtocService};
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
    let rt = Arc::new(runtime::Builder::new_multi_thread()
        .enable_all()
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

#[no_mangle]
pub extern fn initialize_with_manifest(app_manifest: *const c_char, chunk_manifest: *const c_char, cb: extern fn(state: *mut DownloaderState, err: u32)) {
    let rt = Arc::new(runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap());

    let app_manifest = get_string(app_manifest);
    let chunk_manifest = get_string(chunk_manifest);

    let service = match ServiceState::from_manifests(&app_manifest, &chunk_manifest) {
        Ok(res) => res,
        Err(err) => {
            set_last_error(format!("{}", err));
            cb(std::ptr::null_mut(), err.get_code());
            return;
        }
    };

    let state = DownloaderState {
        runtime: rt,
        service: Some(Arc::new(service)),
    };

    cb(Box::into_raw(Box::new(state)), 0);
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
pub extern fn get_pak(ptr: *mut DownloaderState, rfile: *const c_char, cb: extern fn(pak: *mut UtocService, err: u32)) {
    let state = unsafe {
        assert!(!ptr.is_null());
        &*ptr
    };
    let file = get_string(rfile);

    let service = match &state.service {
        Some(data) => Arc::clone(&data),
        None => {
            cb(std::ptr::null_mut(), 13);
            return;
        },
    };

    state.runtime.spawn(async move {
        match service.get_utoc(&file).await {
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
pub extern fn download_file(ptr: *mut DownloaderState, rpak: *const c_char, rfile: *const c_char, cb: extern fn(err: u32)) {
    let state = unsafe {
        assert!(!ptr.is_null());
        &*ptr
    };
    let pak = get_string(rpak);
    let file = get_string(rfile);

    let service = match &state.service {
        Some(data) => Arc::clone(&data),
        None => {
            cb(13);
            return;
        },
    };

    state.runtime.spawn(async move {
        match service.download_file(pak, file).await {
            Ok(_) => cb(0),
            Err(e) => {
                set_last_error(format!("Error: {}", e));
                cb(14);
                return;
            },
        }
    });
}

#[no_mangle]
pub extern fn get_pak_mount(ptr: *mut UtocService) -> *mut c_char {
    let pak = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let c_str = CString::new(pak.get_mount_point()).unwrap();
    c_str.into_raw()
}

#[no_mangle]
pub extern fn get_file_names(ptr: *mut UtocService) -> *mut VecStringHead {
    let pak = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    Box::into_raw(Box::new(VecStringHead {
        contents: pak.get_file_list().clone(),
        index: 0,
    }))
}

#[no_mangle]
pub extern fn get_id_list(ptr: *mut UtocService) -> *mut VecStringHead {
    let pak = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    Box::into_raw(Box::new(VecStringHead {
        contents: pak.get_id_list(),
        index: 0,
    }))
}

#[repr(C)]
pub struct FileDataReturn {
    pub content: *mut c_char,
    pub err: u32,
}

#[no_mangle]
pub extern fn get_file_data(rtptr: *mut DownloaderState, pakptr: *mut UtocService, rfile: *const c_char, cb: extern fn (data: *mut u8, length: u32, err: u32)) {
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
        match pak.get_file(&file).await {
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
pub extern fn free_pak(ptr: *mut UtocService) {
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
