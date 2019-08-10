mod utils;
mod plane;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use alloc::alloc::{Layout, alloc, dealloc};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern crate serde_wasm_bindgen;
extern crate png;
// extern crate image;
extern crate meshopt;
extern crate alloc;


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(v: String);

    #[wasm_bindgen(js_namespace = console, js_name=log)]
    fn log_js(v: &JsValue);

    // #[wasm_bindgen(js_namespace = console, js_name=log)]
    // fn log_vec(v: &);
}

pub fn png2elevation_rs(png_bytes: &[u8]) -> Vec<f32> {
    // decode raw bytes and check we have a 256x256px image
    let decoder = png::Decoder::new(png_bytes);
    let (info, mut reader) = decoder.read_info().unwrap();
    assert_eq!(info.buffer_size(), 256 * 256 * 3);

    // Allocate the output buffer.
    let mut buf = vec![0; info.buffer_size()];
    // Read the next frame. Currently this function should only called once.
    reader.next_frame(&mut buf).unwrap();


    // let buf = match image::load_from_memory(png_bytes) {
    //     Ok(i) => i.to_rgb(),
    //     Err(e) => {
    //         log(e.to_string());
    //         return JsValue::null();
    //     }
    // };

    buf.chunks(3)
        .map(|rgb| rgb[0] as f32 * 256.0 + rgb[1] as f32 + rgb[2] as f32 / 256.0 - 32768.0)
        .collect()
}

#[wasm_bindgen]
pub fn png2elevation(png_bytes: &[u8]) -> JsValue {
    let elevation = png2elevation_rs(png_bytes);
    serde_wasm_bindgen::to_value(&elevation).unwrap()
}

static mut LAYOUT: Option<Layout> = None;

#[wasm_bindgen]
pub extern fn _Znwm(size: usize) -> *mut std::ffi::c_void {
    panic!("error: set meshopt allocator")
}

#[wasm_bindgen]
pub extern fn _ZdlPv(ptr: *mut std::ffi::c_void) {
    panic!("error: set meshopt allocator");
}

unsafe extern fn meshopt_alloc(size: usize) -> *mut std::ffi::c_void{
    LAYOUT = Some(Layout::from_size_align(size, 1).unwrap());
    alloc(LAYOUT.expect("alloc incorrect layout")) as *mut std::ffi::c_void
}

unsafe extern fn meshopt_dealloc(ptr: *mut std::ffi::c_void) {
    dealloc(ptr as *mut u8, LAYOUT.unwrap());
}

#[wasm_bindgen]
pub extern fn __assert_fail(a: u32, b: u32, c: u32, d: u32) {
    log(format!("{} {} {} {}", &a, &b, &c, &d));
}

fn simplify(
    indices: &[u32],
    vertices: &[f32],
    target_count: usize,
    target_error: f32,
) -> Vec<u32> {

    let positions = vertices.as_ptr() as *const u8;
    let mut index_result: Vec<u32> = vec![0; indices.len()];
    // let mut index_result: Vec<u32> = Vec::with_capacity(indices.len());
    let index_count = unsafe {
        meshopt::ffi::meshopt_simplify(
            index_result.as_mut_ptr() as *mut ::std::os::raw::c_uint,
            indices.as_ptr() as *const ::std::os::raw::c_uint,
            indices.len(),
            positions as *const f32,
            vertices.len(),
            12,
            target_count,
            target_error,
        )
    };
    index_result.resize(index_count, 0u32);
    index_result
}

fn optimize(index: &[u32], position: &[f32]) -> Vec<f32> {
    let mut position_result: Vec<f32> = vec![0f32; position.len()];
    let position_count = unsafe {
        meshopt::ffi::meshopt_optimizeVertexFetch(
            position_result.as_mut_ptr() as *mut std::ffi::c_void,
            index.as_ptr() as *mut ::std::os::raw::c_uint,
            index.len(),
            position.as_ptr() as *const std::ffi::c_void,
            position.len(),
            12
        )
    };
    position_result.resize(position_count * 3, 0f32);
    position_result
}

#[wasm_bindgen]
pub fn png2mesh(png_bytes: &[u8], size: f32, segments: u8) -> JsValue {
    let heightmap = png2elevation_rs(png_bytes);

    let (position, index) = plane::build_tile_mesh(size, segments, heightmap);

    // serde_wasm_bindgen::to_value(&(position, index)).unwrap()

    // TODO simplify mesh
    // first: set meshopt allocator
    unsafe {
        meshopt::ffi::meshopt_setAllocator(
            Some(meshopt_alloc),
            Some(meshopt_dealloc)
        );
    }
    // simplifyu the mesh
    let new_index = simplify(
        &index, &position,
        (index.len() as f32 * 0.2) as usize, 0.01
    );
    // optimize the vertices as some vertices are now unused
    let new_position = optimize(&new_index, &position);

    // compute uvs
    let uv: Vec<f32> = new_position.chunks(3).map(
        |xyz| vec![
            (xyz[0] + size / 2.0) / size,
            (xyz[1] + size / 2.0) / size
        ]
    ).flatten().collect();

    serde_wasm_bindgen::to_value(&(new_position, new_index, uv)).unwrap()
}

#[wasm_bindgen]
pub fn init() {
    utils::set_panic_hook();
}
