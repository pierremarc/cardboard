use cairo;
use cairo_sys;
use libc::c_void;
use std::mem;

static IMAGE_SURFACE_DATA: () = ();

unsafe extern "C" fn unbox<T>(data: *mut c_void) {
    let data: Box<T> = Box::from_raw(data as *mut T);
    drop(data);
}

unsafe fn set_user_data<K, T>(
    surface: &cairo::Surface,
    key: &K,
    data: Box<T>,
) -> Result<(), cairo::Status> {
    let ptr: *mut T = Box::into_raw(data);

    assert_eq!(mem::size_of::<*mut c_void>(), mem::size_of_val(&ptr));

    let status = cairo_sys::cairo_surface_set_user_data(
        surface.as_ref().to_raw_none(),
        key as *const _ as *mut _,
        ptr as *mut c_void,
        Some(unbox::<T>),
    );
    match status {
        cairo::Status::Success => Ok(()),
        x => Err(x),
    }
}

pub fn create_for_data_unsafe<D: AsMut<[u8]> + Send>(
    data: D,
    format: cairo::Format,
    width: i32,
    height: i32,
    stride: i32,
) -> Result<cairo::ImageSurface, cairo::Status> {
    let mut data: Box<AsMut<[u8]> + Send> = Box::new(data);

    let (ptr, len) = {
        let mut data = (*data).as_mut();

        (data.as_mut_ptr(), data.len())
    };

    assert!(len >= (height * stride) as usize);
    unsafe {
        let r = cairo::ImageSurface::from_raw_full(cairo_sys::cairo_image_surface_create_for_data(
            ptr, format, width, height, stride,
        ));
        match r {
            Ok(surface) => match set_user_data(&surface, &IMAGE_SURFACE_DATA, Box::new(data)) {
                Ok(_) => Ok(surface),
                Err(x) => Err(x),
            },
            Err(status) => Err(status),
        }
    }
}
