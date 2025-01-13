use core::ffi::c_void;
use std::rc::Rc;
use gdk_sys::gdk_gl_context_get_window;
use glium::backend::Facade;
use glium::implement_vertex;
use glium::index::PrimitiveType;
use glium::program;
use glium::Surface;
use glium::SwapBuffersError;
use irondash_engine_context::EngineContext;
use irondash_texture::{
    BoxedGLTexture, BoxedPixelData, GLTextureProvider, PayloadProvider, SimplePixelData, Texture,
};

use super::GlSource;

struct GLAreaBackend {
    gdk_gl_ctx: *mut gdk_sys::GdkGLContext ,
    width: u32,
    height: u32,
}


struct GLTextureSource {
    gdk_context: GLAreaBackend,
    gl_context: Rc<glium::backend::Context>,
    gl_program: glium::Program,
}



struct Foobar{
    ctx: Rc<glium::backend::Context>,
}


impl Facade for Foobar {
    fn get_context(&self) -> &Rc<glium::backend::Context> {
        &self.ctx
    }
}

unsafe impl glium::backend::Backend for GLAreaBackend {
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        // GTK swaps the buffers after each "render" signal itself
        Ok(())
    }
    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        gl_loader::get_proc_address(symbol) as *const _
    }
    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn is_current(&self) -> bool {
        // GTK makes it current itself on each "render" signal
        true
    }
    unsafe fn make_current(&self) {
        unsafe { gdk_sys::gdk_gl_context_make_current(self.gdk_gl_ctx) };
    }
}

impl GLAreaBackend {
    fn new(glarea: *mut gdk_sys::GdkGLContext, width: u32, height: u32) -> Self {
        Self { gdk_gl_ctx: glarea , width, height}
    }
}


fn init() -> anyhow::Result<()> {
    gl_loader::init_gl();
    Ok(())
}


pub fn render_gl(engine_handle: i64) -> anyhow::Result<()>{
    {

        let engine = EngineContext::get().unwrap();
        let fl_view = engine.get_flutter_view(engine_handle).unwrap();
        let fl_view = unsafe { std::mem::transmute(fl_view) };
        let gtk_widget = unsafe {
            std::mem::transmute(gobject_sys::g_type_check_instance_cast(
                fl_view,
                gtk_sys::gtk_widget_get_type(),
            ))
        };

        let window = unsafe { gtk_sys::gtk_widget_get_parent_window(gtk_widget) };
        let mut error: *mut glib_sys::GError = std::ptr::null_mut();
        let error_ptr: *mut *mut glib_sys::GError = &mut error;
        let gdk_context = unsafe { gdk_sys::gdk_window_create_gl_context(window, error_ptr) };

        unsafe { gdk_sys::gdk_gl_context_make_current(gdk_context) };

        // load gl
        gl_loader::init_gl();


        let texture = Texture::new_with_provider(engine_handle, Arc::new(provider))?;



        // create glium context
        let context = unsafe {
            glium::backend::Context::new(
                GLAreaBackend::new(gdk_context, 800, 600),
                true,
                glium::debug::DebugCallbackBehavior::DebugMessageOnError,
            )
            .unwrap()
        };
        let fb = Foobar{ctx: context};
        let program = get_program(&fb)?;

    

    }
    Ok(())
}


#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}
implement_vertex!(Vertex, position, color);


fn get_program(display: &dyn Facade) -> anyhow::Result<glium::Program>{
    let vertex_buffer = {
        glium::VertexBuffer::new(
            display,
            &[
                Vertex {
                    position: [-0.5, -0.5],
                    color: [0.0, 1.0, 0.0],
                },
                Vertex {
                    position: [0.0, 0.5],
                    color: [0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [0.5, -0.5],
                    color: [1.0, 0.0, 0.0],
                },
            ],
        )
        .unwrap()
    };
            // building the index buffer
            let index_buffer =
            glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();

        // compiling shaders and linking them together
        let program = program!(display,
            100 => {
                vertex: "
                    #version 100

                    uniform lowp mat4 matrix;

                    attribute lowp vec2 position;
                    attribute lowp vec3 color;

                    varying lowp vec3 vColor;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0) * matrix;
                        vColor = color;
                    }
                ",

                fragment: "
                    #version 100
                    varying lowp vec3 vColor;

                    void main() {
                        gl_FragColor = vec4(vColor, 1.0);
                    }
                ",
            },
        )?;
        Ok(program)

}