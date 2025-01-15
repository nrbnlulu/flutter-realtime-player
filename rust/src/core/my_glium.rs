use std::{sync::Arc, thread};

use glow::{
    HasContext, NativeProgram, TEXTURE_2D
};
use irondash_texture::{BoxedGLTexture, GLTextureProvider, PayloadProvider, Texture};
use simple_log::info;

pub struct MyGdkWrapper(*mut gdk_sys::GdkGLContext);

impl MyGdkWrapper {
    pub fn as_gdk(&self) -> *mut gdk_sys::GdkGLContext {
        self.0
    }
}

unsafe impl Send for MyGdkWrapper {}
unsafe impl Sync for MyGdkWrapper {}


struct GLTextureSource {
    width: i32,
    height: i32,
    gl_context: glow::Context,
    texture_name: Option<u32>,
}

impl GLTextureSource {


    pub fn init_gl_context_from_gdk(engine_handle: i64) -> anyhow::Result<Self> {


        gl_loader::init_gl();
        let gl_context = unsafe {
            glow::Context::from_loader_function(|s| {
                std::mem::transmute(gl_loader::get_proc_address(s))
            })
        };
        if let Some(texture) = unsafe { gl_context.create_texture().ok() }{
            let texture_name = texture.0.get();
            unsafe {
                gl_context.bind_texture(TEXTURE_2D, Some(texture));
                gl_context.tex_image_2d(
                    TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    200,
                    500,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&vec![0, 255, 0, 255].repeat(200 * 500))),
                );
                gl_context.bind_texture(TEXTURE_2D, Some(texture));
            }
    
    
    
            unsafe {
                gdk_sys::gdk_gl_context_clear_current();
            }
           return Ok(Self {
                width: 400,
                height: 400,
                gl_context,
                texture_name: Some(texture_name),
            })
        }
        else {
           return Err(anyhow::anyhow!("Failed to create texture"))
        }


    }

    /// setup some state for rendering. not sure why he needed that.
    pub fn init_gl_state(&mut self, width: u32, height: u32, texture_id: i64) -> Result<(), String> {
        Ok(())
    }

    fn init_shaders(gl: &glow::Context) -> Option<NativeProgram> {
        let shader_version = "#version 410";
        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                r#"const vec2 verts[3] = vec2[3](
            vec2(0.5f, 1.0f),
            vec2(0.0f, 0.0f),
            vec2(1.0f, 0.0f)
        );
        out vec2 vert;
        void main() {
            vert = verts[gl_VertexID];
            gl_Position = vec4(vert - 0.5, 0.0, 1.0);
        }"#,
                r#"precision mediump float;
        in vec2 vert;
        out vec4 color;
        void main() {
            color = vec4(vert, 0.5, 1.0);
        }"#,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let mut shaders = Vec::with_capacity(shader_sources.len());

            for (shader_type, shader_source) in shader_sources.iter() {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            Some(program)
        }
    }
}
struct GLTexture {
    pub target: u32,
    pub name: u32,
    pub width: i32,
    pub height: i32,
}

impl GLTexture {
    fn new(target: u32, name: u32, width: i32, height: i32) -> Self {
        Self {
            target,
            name,
            width,
            height,
        }
    }
}

impl GLTextureProvider for GLTexture {
    fn get(&self) -> irondash_texture::GLTexture {
        info!("Returning GLTexture");
        irondash_texture::GLTexture {
            target: self.target,
            name: &self.name,
            width: self.width,
            height: self.height,
        }
    }
}


impl PayloadProvider<BoxedGLTexture> for GLTextureSource {
    fn get_payload(&self) -> BoxedGLTexture {
        //let rng = fastrand::Rng::new();



        let ret = Box::new(GLTexture {
            height: self.width,
            width: self.height,
            target: TEXTURE_2D,
            name: self.texture_name.expect("Texture name not set"),
        });
        ret

    }

    //fn destroy(&self) {
    //    gl.delete_program(self.gl_program);
    //    gl.delete_vertex_array(self.gl_vertexarray);
    //}
}


fn init() -> anyhow::Result<()> {
    gl_loader::init_gl();
    Ok(())
}



pub fn create_ogl_texture(engine_handle: i64) -> anyhow::Result<i64> {

    let provider = Arc::new(GLTextureSource::init_gl_context_from_gdk(engine_handle).unwrap());
    let texture = Texture::new_with_provider(engine_handle, provider.clone())?;
    let id = texture.id();
    let sendable_texture = texture.into_sendable_texture(); 
    thread::spawn(move || {
        loop{
            sendable_texture.mark_frame_available();
            thread::sleep(std::time::Duration::from_secs(1));
        }
    });
    Ok(id)
}
