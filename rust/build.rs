use std::env;
use std::fs::File;
use std::path::PathBuf;

use cfg_aliases::cfg_aliases;
use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};

fn main() {
    #[cfg(target_os = "linux")]
    {
        let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

        println!("cargo:rerun-if-changed=build.rs");

        let mut file = File::create(dest.join("gl_bindings.rs")).unwrap();
        Registry::new(
            Api::Gles2,
            (3, 0),
            Profile::Core,
            Fallbacks::All,
            ["GL_OES_EGL_image"],
        )
        .write_bindings(StructGenerator, &mut file)
        .unwrap();
    }
}
