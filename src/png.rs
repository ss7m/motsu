#![allow(dead_code)]
use crate::image::*;
use crate::pixel::*;
use libc::{c_char, c_int, c_void, fclose, fopen, fread, size_t, FILE};
use std::ffi::CString;
use std::ptr;
use std::slice;

const PNG_TRANSFORM_IDENTITY: c_int = 0x0;
const PNG_TRANSFORM_STRIP_16: c_int = 0x1;
const PNG_TRANSFORM_PACKING: c_int = 0x4;
const PNG_TRANSFORM_GRAY_TO_RGB: c_int = 0x2000;

const PNG_COLOR_MASK_PALETTE: u8 = 1;
const PNG_COLOR_MASK_COLOR: u8 = 2;
const PNG_COLOR_MASK_ALPHA: u8 = 4;

//const PNG_COLOR_TYPE_GRAY: u8 = 0;
const PNG_COLOR_TYPE_PALETTE: u8 = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
const PNG_COLOR_TYPE_RGB: u8 = PNG_COLOR_MASK_COLOR;
const PNG_COLOR_TYPE_RGBA: u8 = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
//const PNG_COLOR_TYPE_GRAY_ALPHA: u8 = PNG_COLOR_MASK_ALPHA;

const PNG_INTERLACE_NONE: c_int = 0;
const PNG_COMPRESSION_TYPE_DEFAULT: c_int = 0;
const PNG_FILTER_TYPE_DEFAULT: c_int = 0;

#[allow(non_camel_case_types)]
#[repr(transparent)]
struct c_png_struct(c_void);

#[allow(non_camel_case_types)]
#[repr(transparent)]
struct c_png_info(c_void);

#[link(name = "png")]
extern "C" {
    fn png_sig_cmp(sig: *const u8, start: size_t, num_to_check: size_t) -> c_int;
    fn png_set_sig_bytes(png_struct: *mut c_png_struct, num_bytes: c_int);
    fn png_create_read_struct(
        version: *const c_char,
        error_ptr: *mut u8,
        error_fn: *mut u8,
        warning_fn: *mut u8,
    ) -> *mut c_png_struct;

    fn png_create_write_struct(
        version: *const c_char,
        error_ptr: *mut u8,
        error_fn: *mut u8,
        warning_fn: *mut u8,
    ) -> *mut c_png_struct;

    fn png_create_info_struct(png_struct: *mut c_png_struct) -> *mut c_png_info;

    fn png_destroy_read_struct(
        png_structpp: *mut *mut c_png_struct,
        png_infopp: *mut *mut c_png_info,
        png_endpp: *mut *mut c_png_info,
    );

    fn png_init_io(png_struct: *mut c_png_struct, file: *mut FILE);

    fn png_read_png(
        png_sturct: *mut c_png_struct,
        png_info: *mut c_png_info,
        transforms: c_int,
        params: *mut c_void,
    );

    fn png_get_image_width(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u32;
    fn png_get_image_height(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u32;
    fn png_get_bit_depth(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u8;
    fn png_get_color_type(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u8;
    fn png_get_rows(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> *mut *mut u8;

    fn png_set_rows(png_struct: *mut c_png_struct, png_info: *mut c_png_info, rows: *mut *mut u8);

    fn png_set_IHDR(
        png_struct: *mut c_png_struct,
        png_info: *mut c_png_info,
        width: u32,
        height: u32,
        bit_depth: c_int,
        color_type: c_int,
        interlace_type: c_int,
        compression_type: c_int,
        filter_method: c_int,
    );

    fn png_write_png(
        png_struct: *mut c_png_struct,
        png_info: *mut c_png_info,
        transforms: c_int,
        params: *mut c_void,
    );
}

fn check_if_png(file: *mut FILE) -> bool {
    let mut bytes: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
    let bytes = bytes.as_mut_ptr();
    unsafe {
        fread(bytes as *mut c_void, 1, 8, file);
        png_sig_cmp(bytes, 0, 8) == 0
    }
}

struct PNG {
    png_struct: *mut c_png_struct,
    png_info: *mut c_png_info,
    filep: Option<*mut FILE>,
}

impl Drop for PNG {
    fn drop(&mut self) {
        unsafe {
            png_destroy_read_struct(&mut self.png_struct, &mut self.png_info, ptr::null_mut())
        };
        if let Some(filep) = self.filep {
            if !filep.is_null() {
                unsafe { fclose(filep) };
            }
        }
    }
}

impl PNG {
    fn new(file_name: &str) -> Result<PNG, String> {
        let version = CString::new("1.6.37").expect("CString::new failed");
        let png_struct = unsafe {
            png_create_read_struct(
                version.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if png_struct.is_null() {
            return Err("Error creating png struct".to_string());
        }

        let png_info = unsafe { png_create_info_struct(png_struct) };

        if png_info.is_null() {
            return Err("Error creating info struct".to_string());
        }

        let mut png = PNG {
            png_struct,
            png_info,
            filep: None,
        };

        if png.read_file(file_name) {
            Ok(png)
        } else {
            Err(format!("{} is not a png file or does not exist", file_name))
        }
    }

    fn read_file(&mut self, file_name: &str) -> bool {
        let file_name = CString::new(file_name).expect("CString::new failed");
        let mode = CString::new("rb").expect("CString::new failed");
        let filep = unsafe { fopen(file_name.as_ptr(), mode.as_ptr()) };

        if filep.is_null() {
            return false;
        } else if !check_if_png(filep) {
            unsafe { fclose(filep) };
            return false;
        }

        unsafe {
            png_init_io(self.png_struct, filep);
            png_set_sig_bytes(self.png_struct, 8);
            png_read_png(
                self.png_struct,
                self.png_info,
                PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_PACKING | PNG_TRANSFORM_GRAY_TO_RGB,
                ptr::null_mut(),
            );
        }

        true
    }

    fn get_image(self) -> Option<Image<RGBA>> {
        let color_type = unsafe { png_get_color_type(self.png_struct, self.png_info) };
        let has_alpha = match color_type {
            PNG_COLOR_TYPE_RGB => false,
            PNG_COLOR_TYPE_RGBA => true,
            _ => return None,
        };

        let height = unsafe { png_get_image_height(self.png_struct, self.png_info) } as usize;
        let width = unsafe { png_get_image_width(self.png_struct, self.png_info) } as usize;
        let rows = unsafe {
            let rows = png_get_rows(self.png_struct, self.png_info);
            slice::from_raw_parts(rows, height)
        };

        let row_size = width * if has_alpha { 4 } else { 3 };
        let mut data = Vec::with_capacity(row_size * height);
        for &row in rows {
            data.extend_from_slice(unsafe { slice::from_raw_parts(row, row_size) })
        }

        Some(if has_alpha {
            Image::new(height, width, data)
        } else {
            let image: Image<RGB> = Image::new(height, width, data);
            image.convert()
        })
    }
}

pub fn load_image_from_png(file_name: &str) -> Option<Image<RGBA>> {
    PNG::new(file_name).ok().and_then(PNG::get_image)
}
