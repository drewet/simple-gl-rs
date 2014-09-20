#![feature(phase)]
#![feature(unsafe_destructor)]
#![unstable]
#![deny(missing_doc)]

/*!
Easy-to-use high-level OpenGL3+ wrapper.

# Initialization

This library defines the `DisplayBuild` trait which is curently implemented only on
`gl_init::WindowBuilder`.

Initialization is done by creating a `WindowBuilder` and calling `build_simple_gl`.

```no_run
extern crate gl_init;
extern crate simple_gl;

fn main() {
    use simple_gl::DisplayBuild;

    let display = gl_init::WindowBuilder::new()
        .with_dimensions(1024, 768)
        .with_title("Hello world".to_string())
        .build_simple_gl().unwrap();
}
```

The `display` object is the most important object of this library.

The window where you are drawing on will produce events. They can be received by calling
`display.poll_events()`.

# Drawing

Drawing something requires three elements:

 - A vertex buffer, which contains the vertices of the shape that you wish to draw.
 - An index buffer, which contains the shapes which connect the vertices.
 - A program that the GPU will execute.

## Vertex buffer

To create a vertex buffer, you must create a struct and add the `#[vertex_format]` attribute to
it. Then simply call `VertexBuffer::new` with a `Vec` of this type.

```no_run
# #![feature(phase)]
# #[phase(plugin)]
# extern crate simple_gl_macros;
# extern crate simple_gl;
# fn main() {
#[vertex_format]
#[allow(non_snake_case)]
struct Vertex {
    iPosition: [f32, ..2],
    iTexCoords: [f32, ..2],
}

# let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
let vertex_buffer = simple_gl::VertexBuffer::new(&display, vec![
    Vertex { iPosition: [-1.0, -1.0], iTexCoords: [0.0, 1.0] },
    Vertex { iPosition: [-1.0,  1.0], iTexCoords: [0.0, 0.0] },
    Vertex { iPosition: [ 1.0,  1.0], iTexCoords: [1.0, 0.0] },
    Vertex { iPosition: [ 1.0, -1.0], iTexCoords: [1.0, 1.0] }
]);
# }
```

## Index buffer

Creating an index buffer is done by calling `build_index_buffer` with an array containing
the indices from the vertex buffer.

```no_run
# let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
let index_buffer = simple_gl::IndexBuffer::new(&display, simple_gl::TrianglesList,
    &[0u8, 1, 2, 0, 2, 3]);
```

## Program

```no_run
static VERTEX_SRC: &'static str = "
    #version 110

    uniform mat4 uMatrix;

    attribute vec2 iPosition;
    attribute vec2 iTexCoords;

    varying vec2 vTexCoords;

    void main() {
        gl_Position = vec4(iPosition, 0.0, 1.0) * uMatrix;
        vTexCoords = iTexCoords;
    }
";

static FRAGMENT_SRC: &'static str = "
    #version 110
    varying vec2 vTexCoords;

    void main() {
        gl_FragColor = vec4(vTexCoords.x, vTexCoords.y, 0.0, 1.0);
    }
";

# let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
let program = simple_gl::Program::new(&display, VERTEX_SRC, FRAGMENT_SRC, None).unwrap();
```

The `attribute`s or `in` variables in the vertex shader must match the names of the elements
of the `#[vertex_format]` structure.

The `Result` returned by `build_program` will report any compilation or linking error.

The last step is to call `build_uniforms` on the program. Doing so does not consume the program,
so you can call `build_uniforms` multiple times on the same program.

```no_run
# let program: simple_gl::Program = unsafe { std::mem::uninitialized() };
let mut uniforms = program.build_uniforms();

uniforms.set_value("uMatrix", [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0f32]
]);
```

## Drawing

Draw by calling `display.draw()`. This function call will return a `Target` object which can
be used to draw things.

Buffers are cleared when the `Target` is created, and swapped when is it destroyed.

Once you are done drawing, you can `target.finish()` or let it go out of the scope.

```no_run
# let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
# let vertex_buffer: simple_gl::VertexBuffer<u8> = unsafe { std::mem::uninitialized() };
# let index_buffer: simple_gl::IndexBuffer = unsafe { std::mem::uninitialized() };
# let uniforms: simple_gl::ProgramUniforms = unsafe { std::mem::uninitialized() };
let mut target = display.draw();
target.draw(&(&vertex_buffer, &index_buffer, &uniforms));
target.finish();
```

*/

#[phase(plugin)]
extern crate compile_msg;

#[phase(plugin)]
extern crate gl_generator;

extern crate gl_init;
extern crate libc;
extern crate native;
extern crate time;

#[doc(hidden)]
pub use data_types::GLDataTuple;

pub use index_buffer::IndexBuffer;
pub use vertex_buffer::{VertexBuffer, VertexBindings, VertexFormat};
pub use texture::Texture;

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

mod context;
mod data_types;
mod index_buffer;
mod texture;
mod vertex_buffer;

#[cfg(target_os = "windows")]
#[cfg(target_os = "linux")]
#[cfg(target_os = "macos")]
mod gl {
    generate_gl_bindings!("gl", "core", "3.3", "struct")
}

#[cfg(target_os = "android")]
mod gl {
    pub use self::Gles2 as Gl;
    generate_gl_bindings!("gles2", "core", "2.0", "struct")
}

#[cfg(not(target_os = "windows"), not(target_os = "linux"), not(target_os = "macos"), not(target_os = "android"))]
compile_error!("This platform is not supported")

/// Something that can be drawn.
pub trait Draw {
    /// Draws the object on the specified target.
    fn draw(&self, &mut Target);
}

/// Types of primitives.
#[allow(missing_doc)]
#[experimental = "Will be replaced soon"]
pub enum PrimitiveType {
    PointsList,
    LinesList,
    LinesListAdjacency,
    LineStrip,
    LineStripAdjacency,
    TrianglesList,
    TrianglesListAdjacency,
    TriangleStrip,
    TriangleStripAdjacency,
    TriangleFan
}

impl PrimitiveType {
    #[cfg(target_os = "windows")]
    #[cfg(target_os = "linux")]
    #[cfg(target_os = "macos")]
    fn get_gl_enum(&self) -> gl::types::GLenum {
        match *self {
            PointsList => gl::POINTS,
            LinesList => gl::LINES,
            LinesListAdjacency => gl::LINES_ADJACENCY,
            LineStrip => gl::LINE_STRIP,
            LineStripAdjacency => gl::LINE_STRIP_ADJACENCY,
            TrianglesList => gl::TRIANGLES,
            TrianglesListAdjacency => gl::TRIANGLES_ADJACENCY,
            TriangleStrip => gl::TRIANGLE_STRIP,
            TriangleStripAdjacency => gl::TRIANGLE_STRIP_ADJACENCY,
            TriangleFan => gl::TRIANGLE_FAN
        }
    }

    #[cfg(target_os = "android")]
    fn get_gl_enum(&self) -> gl::types::GLenum {
        match *self {
            PointsList => gl::POINTS,
            LinesList => gl::LINES,
            LineStrip => gl::LINE_STRIP,
            TrianglesList => gl::TRIANGLES,
            TriangleStrip => gl::TRIANGLE_STRIP,
            TriangleFan => gl::TRIANGLE_FAN,
            _ => fail!("Not supported by GLES")
        }
    }
}

/// Function that the GPU will use for blending.
pub enum BlendingFunction {
    /// Always replace the destination pixel by the source.
    AlwaysReplace,

    /// Linear interpolation of the source pixel by the source pixel's alpha.
    LerpBySourceAlpha,

    /// Linear interpolation of the source pixel by the destination pixel's alpha.
    LerpByDestinationAlpha
}

/// Culling mode.
/// 
/// Describes how triangles could be filtered before the fragment part.
pub enum BackfaceCullingMode {
    /// All triangles are always drawn.
    CullingDisabled,

    /// Triangles whose vertices are counter-clock-wise won't be drawn.
    CullCounterClockWise,

    /// Triangles whose indices are clock-wise won't be drawn.
    CullClockWise
}

/// Function to use for out-of-bounds samples.
///
/// This is how GL must handle samples that are outside the texture.
pub enum SamplerWrapFunction {
    /// Samples at coord `x + 1` are mapped to coord `x`.
    Repeat,

    /// Samples at coord `x + 1` are mapped to coord `1 - x`.
    Mirror,

    /// Samples at coord `x + 1` are mapped to coord `1`.
    Clamp
}

/// The function that the GPU will use when loading the value of a texel.
pub enum SamplerFilter {
    /// The nearest texel will be loaded.
    Nearest,

    /// All nearby texels will be loaded and their values will be merged.
    Linear
}

/// The function that the GPU will use to determine whether to write over an existing pixel
///  on the target.
pub enum DepthFunction {
    /// Never replace the target pixel.
    /// 
    /// This option doesn't really make sense, but is here for completeness.
    Ignore,

    /// Always replace the target pixel.
    Overwrite,

    /// Replace if the z-value of the source is equal to the destination.
    IfEqual,

    /// Replace if the z-value of the source is different than the destination.
    IfNotEqual,

    /// Replace if the z-value of the source is more than the destination.
    IfMore,

    /// Replace if the z-value of the source is more or equal to the destination.
    IfMoreOrEqual,

    /// Replace if the z-value of the source is less than the destination.
    IfLess,

    /// Replace if the z-value of the source is less or equal to the destination.
    IfLessOrEqual
}

/// A target where things can be drawn.
pub struct Target<'t> {
    display: Arc<DisplayImpl>,
    display_hold: Option<&'t Display>,
    texture: Option<&'t mut Texture>,
    framebuffer: Option<FrameBufferObject>,
    execute_end: Option<proc(&DisplayImpl):Send>,
}

impl<'t> Target<'t> {
    /// Stop drawing on the target.
    pub fn finish(self) {
    }

    /// Draws.
    pub fn draw<D: Draw>(&mut self, object: &D) {
        object.draw(self);
    }
}

impl<'a, 'b, 'c, V> Draw for (&'a VertexBuffer<V>, &'b IndexBuffer, &'c ProgramUniforms) {
    fn draw(&self, target: &mut Target) {
        let &(vertex_buffer, index_buffer, program) = self;

        let fbo_id = target.framebuffer.as_ref().map(|f| f.id);
        let (vb_id, vb_elementssize, vb_bindingsclone) = vertex_buffer::get_clone(vertex_buffer);
        let (ib_id, ib_elemcounts, ib_datatype, ib_primitives) = index_buffer::get_clone(index_buffer);
        let program_id = program.program.id.clone();
        let uniforms_clone = program.clone();

        target.display.context.exec(proc(gl) {
            unsafe {
                gl.BindFramebuffer(gl::FRAMEBUFFER, fbo_id.unwrap_or(0));

                gl.Disable(gl::DEPTH_TEST);
                gl.Enable(gl::BLEND);
                gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

                // binding program
                gl.UseProgram(program_id);

                // binding program uniforms
                {
                    let mut active_texture: uint = 0;
                    for (&location, ref texture) in uniforms_clone.textures.iter() {
                        gl.ActiveTexture(gl::TEXTURE0 + active_texture as u32);
                        gl.BindTexture(texture.bind_point, texture.id);
                        gl.Uniform1i(location, active_texture as i32);
                        active_texture = active_texture + 1;
                    }

                    for (&location, &(ref datatype, ref data)) in uniforms_clone.values.iter() {
                        match *datatype {
                            gl::FLOAT       => gl.Uniform1fv(location, 1, data.as_ptr() as *const f32),
                            gl::FLOAT_MAT4  => gl.UniformMatrix4fv(location, 1, 0, data.as_ptr() as *const f32),
                            _ => fail!("Loading uniforms for this type not implemented")
                        }
                        //gl.Uniform1i(location, active_texture as i32);
                    }
                }

                // binding buffers
                gl.BindBuffer(gl::ARRAY_BUFFER, vb_id);
                gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ib_id);

                // binding vertex buffer
                let mut locations = Vec::new();
                for (name, &(data_type, data_size, data_offset)) in vb_bindingsclone.iter() {
                    let loc = gl.GetAttribLocation(program_id, name.to_c_str().unwrap());
                    locations.push(loc);

                    if loc != -1 {
                        match data_type {
                            gl::BYTE | gl::UNSIGNED_BYTE | gl::SHORT | gl::UNSIGNED_SHORT | gl::INT | gl::UNSIGNED_INT
                                => fail!("Not supported"), // TODO: gl.VertexAttribIPointer(loc as u32, data_size, data_type, vb_elementssize as i32, data_offset as *const libc::c_void),
                            _ => gl.VertexAttribPointer(loc as u32, data_size, data_type, 0, vb_elementssize as i32, data_offset as *const libc::c_void)
                        }
                        
                        gl.EnableVertexAttribArray(loc as u32);
                    }
                }
                
                // drawing
                gl.DrawElements(ib_primitives, ib_elemcounts as i32, ib_datatype, std::ptr::null());

                // disable vertex attrib array
                for l in locations.iter() {
                    gl.DisableVertexAttribArray(l.clone() as u32);
                }
            }
        }).get();
    }
}

#[unsafe_destructor]
impl<'t> Drop for Target<'t> {
    fn drop(&mut self) {
        match self.execute_end.take() {
            Some(f) => f(&*self.display),
            None => ()
        }
    }
}

struct ShaderImpl {
    display: Arc<DisplayImpl>,
    id: gl::types::GLuint,
}

impl Drop for ShaderImpl {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl) {
            gl.DeleteShader(id);
        });
    }
}

/// A combinaison of shaders linked together.
pub struct Program {
    program: Arc<ProgramImpl>
}

impl Program {
    /// Builds a new program.
    ///
    /// A program is a group of shaders linked together.
    ///
    /// # Parameters
    ///
    /// - `vertex_shader`: Source code of the vertex shader.
    /// - `fragment_shader`: Source code of the fragment shader.
    /// - `geometry_shader`: Source code of the geometry shader.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
    /// # let vertex_source = ""; let fragment_source = ""; let geometry_source = "";
    /// let program = simple_gl::Program::new(&display, vertex_source, fragment_source, Some(geometry_source));
    /// ```
    /// 
    #[experimental = "The list of shaders and the result error will probably change"]
    pub fn new(display: &Display, vertex_shader: &str, fragment_shader: &str,
               geometry_shader: Option<&str>) -> Result<Program, String>
    {
        display.build_program(vertex_shader, fragment_shader, geometry_shader)
    }

    /// Creates a new `ProgramUniforms` object.
    ///
    /// A `ProgramUniforms` object is a link between a program and its uniforms values.
    pub fn build_uniforms(&self) -> ProgramUniforms {
        ProgramUniforms {
            display: self.program.display.clone(),
            program: self.program.clone(),
            textures: HashMap::new(),
            values: HashMap::new(),
            uniforms: self.program.uniforms.clone()
        }
    }
}

impl fmt::Show for Program {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("Program #{}", self.program.id)).fmt(formatter)
    }
}

struct ProgramImpl {
    display: Arc<DisplayImpl>,
    #[allow(dead_code)]
    shaders: Vec<Arc<ShaderImpl>>,
    id: gl::types::GLuint,
    uniforms: Arc<HashMap<String, (gl::types::GLint, gl::types::GLenum, gl::types::GLint)>>     // location, type and size of each uniform, ordered by name
}

/// A program which stores values of uniforms.
#[deriving(Clone)]
pub struct ProgramUniforms {
    display: Arc<DisplayImpl>,
    program: Arc<ProgramImpl>,
    textures: HashMap<gl::types::GLint, Arc<texture::TextureImpl>>,
    values: HashMap<gl::types::GLint, (gl::types::GLenum, Vec<char>)>,
    uniforms: Arc<HashMap<String, (gl::types::GLint, gl::types::GLenum, gl::types::GLint)>>     // same as the program's variable
}

impl ProgramUniforms {
    /// Modifies the value of a uniform of the program.
    ///
    /// `uniform_name` must be the name of a uniform in the program.
    /// Nothing happens if the program doesn't contain a uniform with this name.
    /// However the function will fail if the type of data doesn't match the type required
    ///  by the shader source code.
    pub fn set_value<T: data_types::UniformValue>(&mut self, uniform_name: &str, value: T) {
        let &(location, gltype, _) = match self.uniforms.find(&uniform_name.to_string()) {
            Some(a) => a,
            None => return      // the uniform is not used, we ignore it
        };

        if gltype != data_types::UniformValue::get_gl_type(None::<T>) {
            fail!("Type of data passed to set_value must match the type of data requested by the shader")
        }

        let mut data: Vec<char> = Vec::with_capacity(std::mem::size_of_val(&value));
        unsafe { data.set_len(std::mem::size_of_val(&value)); }

        let data_inside = data.as_mut_ptr() as *mut T;
        unsafe { (*data_inside) = value; }

        self.values.insert(location.clone(), (gltype, data));
    }

    /// Modifies the value of a texture uniform of the program.
    ///
    /// `uniform_name` must be the name of a uniform in the program.
    /// Nothing happens if the program doesn't contain a uniform with this name.
    /// However the function will fail if you call this function for a non-texture uniform.
    pub fn set_texture(&mut self, uniform_name: &str, texture: &Texture) {
        let &(location, gltype, _) = match self.uniforms.find(&uniform_name.to_string()) {
            Some(a) => a,
            None => return      // the uniform is not used, we ignore it
        };

        // TODO: fix the check for GLES
        /*match gltype {
            gl::SAMPLER_1D | gl::SAMPLER_2D | gl::SAMPLER_3D | gl::SAMPLER_CUBE |
            gl::SAMPLER_1D_SHADOW | gl::SAMPLER_2D_SHADOW | gl::SAMPLER_1D_ARRAY |
            gl::SAMPLER_2D_ARRAY | gl::SAMPLER_1D_ARRAY_SHADOW | gl::SAMPLER_2D_ARRAY_SHADOW |
            gl::SAMPLER_2D_MULTISAMPLE | gl::SAMPLER_2D_MULTISAMPLE_ARRAY |
            gl::SAMPLER_CUBE_SHADOW | gl::SAMPLER_BUFFER | gl::SAMPLER_2D_RECT |
            gl::SAMPLER_2D_RECT_SHADOW | gl::INT_SAMPLER_1D | gl::INT_SAMPLER_2D |
            gl::INT_SAMPLER_3D | gl::INT_SAMPLER_CUBE | gl::INT_SAMPLER_1D_ARRAY |
            gl::INT_SAMPLER_2D_ARRAY | gl::INT_SAMPLER_2D_MULTISAMPLE |
            gl::INT_SAMPLER_2D_MULTISAMPLE_ARRAY | gl::INT_SAMPLER_BUFFER |
            gl::INT_SAMPLER_2D_RECT | gl::UNSIGNED_INT_SAMPLER_1D | gl::UNSIGNED_INT_SAMPLER_2D |
            gl::UNSIGNED_INT_SAMPLER_3D | gl::UNSIGNED_INT_SAMPLER_CUBE |
            gl::UNSIGNED_INT_SAMPLER_1D_ARRAY | gl::UNSIGNED_INT_SAMPLER_2D_ARRAY |
            gl::UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE |
            gl::UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE_ARRAY | gl::UNSIGNED_INT_SAMPLER_BUFFER |
            gl::UNSIGNED_INT_SAMPLER_2D_RECT
                => (),
            _ => fail!("Trying to bind a texture to a non-texture uniform")
        };*/

        self.textures.insert(location.clone(), texture::get_impl(texture).clone());
    }
}

/// Frame buffer.
struct FrameBufferObject {
    display: Arc<DisplayImpl>,
    id: gl::types::GLuint,
}

impl FrameBufferObject {
    /// Builds a new FBO.
    fn new(display: Arc<DisplayImpl>) -> FrameBufferObject {
        let id = display.context.exec(proc(gl) {
            unsafe {
                let id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenFramebuffers(1, std::mem::transmute(&id));
                id
            }
        }).get();

        FrameBufferObject {
            display: display,
            id: id,
        }
    }
}

impl Drop for FrameBufferObject {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl) {
            unsafe { gl.DeleteFramebuffers(1, [ id ].as_ptr()); }
        });
    }
}

/// Render buffer.
struct RenderBuffer {
    display: Arc<DisplayImpl>,
    id: gl::types::GLuint,
}

impl RenderBuffer {
    /// Builds a new render buffer.
    fn new(display: Arc<DisplayImpl>) -> RenderBuffer {
        let id = display.context.exec(proc(gl) {
            unsafe {
                let id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenRenderbuffers(1, std::mem::transmute(&id));
                id
            }
        }).get();

        RenderBuffer {
            display: display,
            id: id,
        }
    }
}

impl Drop for RenderBuffer {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl) {
            unsafe { gl.DeleteRenderbuffers(1, [ id ].as_ptr()); }
        });
    }
}

/// Objects that can build a `Display` object.
pub trait DisplayBuild {
    /// Build a context and a `Display` to draw on it.
    fn build_simple_gl(self) -> Result<Display, ()>;
}

impl DisplayBuild for gl_init::WindowBuilder {
    fn build_simple_gl(self) -> Result<Display, ()> {
        let window = try!(self.build().map_err(|_| ()));
        let context = context::Context::new(window);

        let gl_version = context.exec(proc(gl) {
            // TODO: not supported by GLES
            (0, 0)
            /*unsafe {
                use std::mem;

                let mut major_version: gl::types::GLint = mem::uninitialized();
                let mut minor_version: gl::types::GLint = mem::uninitialized();

                gl.GetIntegerv(gl::MAJOR_VERSION, &mut major_version);
                gl.GetIntegerv(gl::MINOR_VERSION, &mut minor_version);

                (major_version, minor_version)
            }*/
        }).get();

        Ok(Display {
            context: Arc::new(DisplayImpl {
                context: context,
                gl_version: gl_version,
            }),
        })
    }
}

/// The main object of this library. Controls the whole display.
pub struct Display {
    context: Arc<DisplayImpl>
}

struct DisplayImpl {
    context: context::Context,
    gl_version: (gl::types::GLint, gl::types::GLint),
}

impl Display {
    /// Reads all events received by the window.
    pub fn poll_events(&self) -> Vec<gl_init::Event> {
        self.context.context.recv()
    }

    /// 
    pub fn draw(&self) -> Target {
        Target {
            display: self.context.clone(),
            display_hold: Some(self),
            texture: None,
            framebuffer: None,
            execute_end: Some(proc(context: &DisplayImpl) {
                context.context.swap_buffers();

                context.context.exec(proc(gl) {
                    gl.ClearColor(0.0, 0.0, 0.0, 1.0);
                    gl.Clear(gl::COLOR_BUFFER_BIT);
                });
            }),
        }
    }

    /// See `VertexBuffer::new`
    #[deprecated = "Use VertexBuffer::new"]
    pub fn build_vertex_buffer<T: VertexFormat + 'static + Send>(&self, data: Vec<T>)
        -> VertexBuffer<T>
    {
        VertexBuffer::new(self, data)
    }

    /// See `IndexBuffer::new`
    #[deprecated = "Use IndexBuffer::new"]
    pub fn build_index_buffer<T: data_types::GLDataType>(&self, prim: PrimitiveType, data: &[T]) -> IndexBuffer {
        IndexBuffer::new(self, prim, data)
    }

    /// Builds an individual shader.
    fn build_shader<S: ToCStr>(&self, shader_type: gl::types::GLenum, source_code: S)
        -> Result<Arc<ShaderImpl>, String>
    {
        let source_code = source_code.to_c_str();

        let id_result = self.context.context.exec(proc(gl) {
            unsafe {
                let id = gl.CreateShader(shader_type);

                gl.ShaderSource(id, 1, [ source_code.as_ptr() ].as_ptr(), std::ptr::null());
                gl.CompileShader(id);

                // checking compilation success
                let compilation_success = {
                    let mut compilation_success: gl::types::GLint = std::mem::uninitialized();
                    gl.GetShaderiv(id, gl::COMPILE_STATUS, &mut compilation_success);
                    compilation_success
                };

                if compilation_success == 0 {
                    // compilation error
                    let mut error_log_size: gl::types::GLint = std::mem::uninitialized();
                    gl.GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut error_log_size);

                    let mut error_log: Vec<u8> = Vec::with_capacity(error_log_size as uint);
                    gl.GetShaderInfoLog(id, error_log_size, &mut error_log_size, error_log.as_mut_slice().as_mut_ptr() as *mut gl::types::GLchar);
                    error_log.set_len(error_log_size as uint);

                    let msg = String::from_utf8(error_log).unwrap();
                    return Err(msg)
                }

                Ok(id)
            }
        }).get();

        id_result.map(|id| {
            Arc::new(ShaderImpl {
                display: self.context.clone(),
                id: id
            })
        })
    }

    /// Builds a new texture.
    pub fn build_texture<T: data_types::GLDataTuple>(&self, data: &[T], width: uint, height: uint, depth: uint, array_size: uint)
        -> Texture
    {
        Texture::new(self, data, width, height, depth, array_size)
    }

    #[cfg(target_os = "windows")]
    #[cfg(target_os = "linux")]
    #[cfg(target_os = "macos")]
    fn build_geometry_shader<S: ToCStr>(&self, source_code: S)
        -> Result<Arc<ShaderImpl>, String>
    {
        self.build_shader(gl::GEOMETRY_SHADER, source_code)
    }
    
    #[cfg(target_os = "android")]
    fn build_geometry_shader<S: ToCStr>(&self, source_code: S)
        -> Result<Arc<ShaderImpl>, String>
    {
        Err(format!("Geometry shaders are not supported on this platform"))
    }

    /// See `Program::new`
    #[deprecated = "Use Program::new instead"]
    pub fn build_program(&self, vertex_shader: &str, fragment_shader: &str,
                         geometry_shader: Option<&str>) -> Result<Program, String>
    {
        let mut shaders_store = Vec::new();
        shaders_store.push(try!(self.build_shader(gl::VERTEX_SHADER, vertex_shader)));
        match geometry_shader {
            Some(gs) => shaders_store.push(try!(self.build_geometry_shader(gs))),
            None => ()
        }
        shaders_store.push(try!(self.build_shader(gl::FRAGMENT_SHADER, fragment_shader)));

        let mut shaders_ids = Vec::new();
        for sh in shaders_store.iter() {
            shaders_ids.push(sh.id);
        }

        let id = try!(self.context.context.exec(proc(gl) {
            unsafe {
                let id = gl.CreateProgram();
                if id == 0 {
                    return Err(format!("glCreateProgram failed"));
                }

                // attaching shaders
                for sh in shaders_ids.iter() {
                    gl.AttachShader(id, sh.clone());
                }

                // linking and checking for errors
                gl.LinkProgram(id);
                {   let mut link_success: gl::types::GLint = std::mem::uninitialized();
                    gl.GetProgramiv(id, gl::LINK_STATUS, &mut link_success);
                    if link_success == 0 {
                        match gl.GetError() {
                            gl::NO_ERROR => (),
                            gl::INVALID_VALUE => return Err(format!("glLinkProgram triggered GL_INVALID_VALUE")),
                            gl::INVALID_OPERATION => return Err(format!("glLinkProgram triggered GL_INVALID_OPERATION")),
                            _ => return Err(format!("glLinkProgram triggered an unknown error"))
                        };

                        let mut error_log_size: gl::types::GLint = std::mem::uninitialized();
                        gl.GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut error_log_size);

                        let mut error_log: Vec<u8> = Vec::with_capacity(error_log_size as uint);
                        gl.GetProgramInfoLog(id, error_log_size, &mut error_log_size, error_log.as_mut_slice().as_mut_ptr() as *mut gl::types::GLchar);
                        error_log.set_len(error_log_size as uint);

                        let msg = String::from_utf8(error_log).unwrap();
                        return Err(msg)
                    }
                }

                Ok(id)
            }
        }).get());

        let uniforms = self.context.context.exec(proc(gl) {
            unsafe {
                // reflecting program uniforms
                let mut uniforms = HashMap::new();

                let mut active_uniforms: gl::types::GLint = std::mem::uninitialized();
                gl.GetProgramiv(id, gl::ACTIVE_UNIFORMS, &mut active_uniforms);

                for uniform_id in range(0, active_uniforms) {
                    let mut uniform_name_tmp: Vec<u8> = Vec::with_capacity(64);
                    let mut uniform_name_tmp_len = 63;

                    let mut data_type: gl::types::GLenum = std::mem::uninitialized();
                    let mut data_size: gl::types::GLint = std::mem::uninitialized();
                    gl.GetActiveUniform(id, uniform_id as gl::types::GLuint, uniform_name_tmp_len, &mut uniform_name_tmp_len, &mut data_size, &mut data_type, uniform_name_tmp.as_mut_slice().as_mut_ptr() as *mut gl::types::GLchar);
                    uniform_name_tmp.set_len(uniform_name_tmp_len as uint);

                    let uniform_name = String::from_utf8(uniform_name_tmp).unwrap();
                    let location = gl.GetUniformLocation(id, uniform_name.to_c_str().unwrap());

                    uniforms.insert(uniform_name, (location, data_type, data_size));
                }

                Arc::new(uniforms)
            }
        }).get();


        Ok(Program {
            program: Arc::new(ProgramImpl {
                display: self.context.clone(),
                shaders: shaders_store,
                id: id,
                uniforms: uniforms
            })
        })
    }
}
