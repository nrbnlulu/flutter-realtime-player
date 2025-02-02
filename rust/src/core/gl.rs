#![allow(clippy::all)]
include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

pub use Gles2 as GlFunctions;

unsafe impl Sync for Gles2 {}

use lazy_static::lazy_static;

lazy_static! {
    pub static ref GL: GlFunctions =
        GlFunctions::load_with(|symbol| { gl_loader::get_proc_address(symbol) as *const _ });
}
