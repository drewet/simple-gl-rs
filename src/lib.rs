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
let index_buffer = display.build_index_buffer(simple_gl::TrianglesList,
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
extern crate gl_generator;

extern crate gl_init;
extern crate libc;
extern crate native;
extern crate time;

#[doc(hidden)]
pub use data_types::GLDataTuple;

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

mod context;
mod data_types;

mod gl {
    generate_gl_bindings!("gl", "core", "3.3", "struct")
}

/// Something that can be drawn.
trait Draw {
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

/// A texture usable by OpenGL.
pub struct Texture {
    texture: Arc<TextureImpl>
}

impl Texture {
    /// Returns the width of the texture.
    pub fn get_width(&self) -> uint {
        self.texture.width
    }

    /// Returns the height of the texture, or 1 if the texture is a 1D texture.
    pub fn get_height(&self) -> uint {
        self.texture.height
    }

    /// Returns the depth of the texture, or 1 if the texture is a 1D or 2D texture.
    pub fn get_depth(&self) -> uint {
        self.texture.depth
    }

    /// Returns the number of elements in the texture array, or 1 if the texture is not an array.
    pub fn get_array_size(&self) -> uint {
        self.texture.array_size
    }

    /// Start drawing on this texture.
    pub fn draw(&mut self) -> Target {
        let display = self.texture.display.clone();
        let fbo = FrameBufferObject::new(display.clone());

        // binding the texture to the FBO
        {
            let my_id = self.texture.id.clone();
            let fbo_id = fbo.id;
            self.texture.display.context.exec(proc(gl) {
                gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, fbo_id);
                gl.FramebufferTexture(gl::DRAW_FRAMEBUFFER, gl::COLOR_ATTACHMENT0, my_id, 0);
            });
        }

        // returning the target
        Target {
            display: display,
            display_hold: None,
            texture: Some(self),
            framebuffer: Some(fbo),
            execute_end: None,
        }
    }

    /// Reads the content of the texture.
    ///
    /// Same as `read_mipmap` with `level` as `0`.
    // TODO: draft ; must be checked and turned public
    fn read(&self) -> Vec<u8> {
        self.read_mipmap(0)
    }

    /// Reads the content of one of the mipmaps the texture.
    ///
    /// Returns a 2D array of pixels.
    /// Each pixel has R, G and B components between 0 and 255.
    // TODO: draft ; must be checked and turned public
    fn read_mipmap(&self, level: uint) -> Vec<u8> {
        let bind_point = self.texture.bind_point;
        let id = self.texture.id;
        let buffer_size = self.texture.width * self.texture.height * self.texture.depth *
            self.texture.array_size * 3;

        if level != 0 {
            unimplemented!()
        }

        self.texture.display.context.exec(proc(gl) {
            let mut buffer = Vec::from_elem(buffer_size, 0u8);

            unsafe {
                gl.BindTexture(bind_point, id);
                gl.GetTexImage(bind_point, 0 as gl::types::GLint, gl::RGBA_INTEGER, gl::UNSIGNED_BYTE,
                    buffer.as_mut_ptr() as *mut libc::c_void);
            }

            buffer
        }).get()
    }
}

impl fmt::Show for Texture {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("Texture #{} (dimensions: {}x{}x{})", self.texture.id,
            self.texture.width, self.texture.height, self.texture.depth)).fmt(formatter)
    }
}

struct TextureImpl {
    display: Arc<DisplayImpl>,
    id: gl::types::GLuint,
    bind_point: gl::types::GLenum,
    width: uint,
    height: uint,
    depth: uint,
    array_size: uint
}

impl Drop for TextureImpl {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl) {
            unsafe { gl.DeleteTextures(1, [ id ].as_ptr()); }
        });
    }
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
        let vb_id = vertex_buffer.id.clone();
        let vb_bindingsclone = vertex_buffer.bindings.clone();
        let vb_elementssize = vertex_buffer.elements_size.clone();
        let ib_id = index_buffer.id.clone();
        let ib_primitives = index_buffer.primitives.clone();
        let ib_elemcounts = index_buffer.elements_count.clone();
        let ib_datatype = index_buffer.data_type.clone();
        let program_id = program.program.id.clone();
        let uniforms_clone = program.clone();

        target.display.context.exec(proc(gl) {
            unsafe {
                gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, fbo_id.unwrap_or(0));

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
                                => gl.VertexAttribIPointer(loc as u32, data_size, data_type, vb_elementssize as i32, data_offset as *const libc::c_void),
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
    textures: HashMap<gl::types::GLint, Arc<TextureImpl>>,
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

        match gltype {
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
        };

        self.textures.insert(location.clone(), texture.texture.clone());
    }
}

/// A list of verices loaded in the graphics card's memory.
pub struct VertexBuffer<T> {
    display: Arc<DisplayImpl>,
    id: gl::types::GLuint,
    elements_size: uint,
    bindings: VertexBindings,
}

impl<T: VertexFormat + 'static + Send> VertexBuffer<T> {
    /// Builds a new vertex buffer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #![feature(phase)]
    /// # #[phase(plugin)]
    /// # extern crate simple_gl_macros;
    /// # extern crate simple_gl;
    /// # fn main() {
    /// #[vertex_format]
    /// struct Vertex {
    ///     position: [f32, ..3],
    ///     texcoords: [f32, ..2],
    /// }
    ///
    /// # let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
    /// let vertex_buffer = simple_gl::VertexBuffer::new(&display, vec![
    ///     Vertex { position: [0.0,  0.0, 0.0], texcoords: [0.0, 1.0] },
    ///     Vertex { position: [5.0, -3.0, 2.0], texcoords: [1.0, 0.0] },
    /// ]);
    /// # }
    /// ```
    /// 
    pub fn new(display: &Display, data: Vec<T>) -> VertexBuffer<T> {
        let bindings = VertexFormat::build_bindings(None::<T>);

        let elements_size = { use std::mem; mem::size_of::<T>() };
        let buffer_size = data.len() * elements_size as uint;

        let id = display.context.context.exec(proc(gl) {
            unsafe {
                let mut id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenBuffers(1, &mut id);
                gl.BindBuffer(gl::ARRAY_BUFFER, id);
                gl.BufferData(gl::ARRAY_BUFFER, buffer_size as gl::types::GLsizeiptr,
                    data.as_ptr() as *const libc::c_void, gl::STATIC_DRAW);
                id
            }
        }).get();

        VertexBuffer {
            display: display.context.clone(),
            id: id,
            elements_size: elements_size,
            bindings: bindings
        }
    }
}

impl<T> fmt::Show for VertexBuffer<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("VertexBuffer #{}", self.id)).fmt(formatter)
    }
}

#[unsafe_destructor]
impl<T> Drop for VertexBuffer<T> {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl) {
            unsafe { gl.DeleteBuffers(1, [ id ].as_ptr()); }
        });
    }
}

/// A list of indices loaded in the graphics card's memory.
pub struct IndexBuffer {
    display: Arc<DisplayImpl>,
    id: gl::types::GLuint,
    elements_count: uint,
    data_type: gl::types::GLenum,
    primitives: gl::types::GLenum
}

impl fmt::Show for IndexBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("IndexBuffer #{} (elements: {})", self.id, self.elements_count)).fmt(formatter)
    }
}

impl Drop for IndexBuffer {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl) {
            unsafe { gl.DeleteBuffers(1, [ id ].as_ptr()); }
        });
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

/// For each binding, the data type, number of elements, and offset.
/// Includes the total size.
#[doc(hidden)]
pub type VertexBindings = HashMap<String, (gl::types::GLenum, gl::types::GLint, uint)>;

/// Trait for structures that represent a vertex.
#[doc(hidden)]
pub trait VertexFormat: Copy {
    fn build_bindings(Option<Self>) -> VertexBindings;
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
        Ok(Display {
            context: Arc::new(DisplayImpl {
                context: context,
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

    /// Builds a new index buffer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
    /// let index_buffer = display.build_index_buffer(simple_gl::TrianglesList,
    ///     &[0u8, 1, 2, 1, 3, 4, 2, 4, 3]);
    /// ```
    /// 
    pub fn build_index_buffer<T: data_types::GLDataType>(&self, prim: PrimitiveType, data: &[T]) -> IndexBuffer {
        let elements_size = std::mem::size_of_val(&data[0]);
        let data_size = data.len() * elements_size;
        let data_ptr: *const libc::c_void = data.as_ptr() as *const libc::c_void;

        let id = self.context.context.exec(proc(gl) {
            unsafe {
                let id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenBuffers(1, std::mem::transmute(&id));
                gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, id);
                gl.BufferData(gl::ELEMENT_ARRAY_BUFFER, data_size as gl::types::GLsizeiptr, data_ptr, gl::STATIC_DRAW);
                id
            }
        }).get();

        IndexBuffer {
            display: self.context.clone(),
            id: id,
            elements_count: data.len(),
            data_type: data_types::GLDataType::get_gl_type(None::<T>),
            primitives: prim.get_gl_enum()
        }
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
        let element_components = data_types::GLDataTuple::get_num_elems(None::<T>);

        if width * height * depth * array_size != data.len() {
            fail!("Texture data has different size from width*height*depth*array_size*elemLen");
        }

        let texture_type = if height == 1 && depth == 1 {
            if array_size == 1 { gl::TEXTURE_1D } else { gl::TEXTURE_1D_ARRAY }
        } else if depth == 1 {
            if array_size == 1 { gl::TEXTURE_2D } else { gl::TEXTURE_2D_ARRAY }
        } else {
            gl::TEXTURE_3D
        };

        let data_type = data_types::GLDataTuple::get_gl_type(None::<T>);
        let data_raw: *const libc::c_void = unsafe { std::mem::transmute(data.as_ptr()) };

        let (data_format, data_type) = match (element_components, data_type) {
            (1, f) => (gl::RED, f),
            (2, f) => (gl::RG, f),
            (3, f) => (gl::RGB, f),
            (4, f) => (gl::RGBA, f),
            _ => fail!("unsupported texture type")
        };

        let id = self.context.context.exec(proc(gl) {
            unsafe {
                gl.PixelStorei(gl::UNPACK_ALIGNMENT, if width % 4 == 0 { 4 } else if height % 2 == 0 { 2 } else { 1 });

                let id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenTextures(1, std::mem::transmute(&id));

                gl.BindTexture(texture_type, id);

                gl.TexParameteri(texture_type, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
                if height != 1 || depth != 1 || array_size != 1 {
                    gl.TexParameteri(texture_type, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
                }
                if depth != 1 || array_size != 1 {
                    gl.TexParameteri(texture_type, gl::TEXTURE_WRAP_R, gl::REPEAT as i32);
                }
                gl.TexParameteri(texture_type, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                gl.TexParameteri(texture_type, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as i32);

                if texture_type == gl::TEXTURE_3D || texture_type == gl::TEXTURE_2D_ARRAY {
                    gl.TexImage3D(texture_type, 0, gl::RGBA as i32, width as i32, height as i32, if depth > 1 { depth } else { array_size } as i32, 0, data_format as u32, data_type, data_raw);
                } else if texture_type == gl::TEXTURE_2D || texture_type == gl::TEXTURE_1D_ARRAY {
                    gl.TexImage2D(texture_type, 0, gl::RGBA as i32, width as i32, height as i32, 0, data_format as u32, data_type, data_raw);
                } else {
                    gl.TexImage1D(texture_type, 0, gl::RGBA as i32, width as i32, 0, data_format as u32, data_type, data_raw);
                }

                gl.GenerateMipmap(texture_type);

                id
            }
        }).get();

        Texture {
            texture: Arc::new(TextureImpl {
                display: self.context.clone(),
                id: id,
                bind_point: texture_type,
                width: width,
                height: height,
                depth: depth,
                array_size: array_size
            })
        }
    }

    /// See `Program::new`
    #[deprecated = "Use Program::new instead"]
    pub fn build_program(&self, vertex_shader: &str, fragment_shader: &str,
                         geometry_shader: Option<&str>) -> Result<Program, String>
    {
        let mut shaders_store = Vec::new();
        shaders_store.push(try!(self.build_shader(gl::VERTEX_SHADER, vertex_shader)));
        match geometry_shader {
            Some(gs) => shaders_store.push(try!(self.build_shader(gl::GEOMETRY_SHADER, gs))),
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
