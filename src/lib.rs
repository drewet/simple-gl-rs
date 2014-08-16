#![feature(phase)]
#![unstable]

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

pub enum PrimitiveType {
    PointsList,
    LinesList,
    LineStrip,
    TrianglesList,
    TrianglesListAdjacency,
    TriangleStrip,
    TriangleStripAdjacency,
    TriangleFan
}

pub enum BlendingFunction {
    AlwaysReplace,
    LerpBySourceAlpha,
    LerpByDestinationAlpha
}

/// Culling mode.
/// 
/// Describes how triangles could be filtered before the fragment part.
///
/// - `CullingDisabled`: All triangles are always drawn.
/// - `CullCounterClockWise`: Triangles whose vertices are counter-clock-wise won't be drawn.
/// - `CullClockWise`: Triangles whose indices are clock-wise won't be drawn.
pub enum BackfaceCullingMode {
    CullingDisabled,
    CullCounterClockWise,
    CullClockWise
}

/// Function to use for out-of-bounds samples.
///
/// This is how GL must handle samples that are outside the texture.
///
/// - `Repeat`: Samples at coord `x + 1` are mapped to coord `x`.
/// - `Mirror`: Samples at coord `x + 1` are mapped to coord `1 - x`.
/// - `Clamp`: Samples at coord `x + 1` are mapped to coord `1`.
pub enum SamplerWrapFunction {
    Repeat,
    Mirror,
    Clamp
}

/// 
pub enum SamplerFilter {
    Nearest,
    Linear
}

/// 
pub enum DepthFunction {
    Ignore,
    Overwrite,
    IfEqual,
    IfNotEqual,
    IfMore,
    IfMoreOrEqual,
    IfLess,
    IfLessOrEqual
}

/// The main object of this library. Controls the whole display.
pub struct Display {
    context : Arc<context::Context>
}

pub struct Texture {
    texture: Arc<TextureImpl>
}

impl fmt::Show for Texture {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("Texture #{} (dimensions: {}x{}x{})", self.texture.id,
            self.texture.width, self.texture.height, self.texture.depth)).fmt(formatter)
    }
}

struct TextureImpl {
    display: Arc<context::Context>,
    id: gl::types::GLuint,
    bindPoint: gl::types::GLenum,
    width: uint,
    height: uint,
    depth: uint,
    arraySize: uint
}

struct ShaderImpl {
    display: Arc<context::Context>,
    id: gl::types::GLuint,
}

pub struct Program {
    program: Arc<ProgramImpl>
}

impl fmt::Show for Program {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("Program #{}", self.program.id)).fmt(formatter)
    }
}

struct ProgramImpl {
    display: Arc<context::Context>,
    #[allow(dead_code)]
    shaders: Vec<Arc<ShaderImpl>>,
    id: gl::types::GLuint,
    uniforms: Arc<HashMap<String, (gl::types::GLint, gl::types::GLenum, gl::types::GLint)>>     // location, type and size of each uniform, ordered by name
}

#[deriving(Clone)]
pub struct ProgramUniforms {
    display: Arc<context::Context>,
    program: Arc<ProgramImpl>,
    textures: HashMap<gl::types::GLint, Arc<TextureImpl>>,
    values: HashMap<gl::types::GLint, (gl::types::GLenum, Vec<char>)>,
    uniforms: Arc<HashMap<String, (gl::types::GLint, gl::types::GLenum, gl::types::GLint)>>     // same as the program's variable
}

pub struct VertexBuffer {
    display: Arc<context::Context>,
    id: gl::types::GLuint,
    elements_size: uint,
    bindings: VertexBindings,
}

impl fmt::Show for VertexBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("VertexBuffer #{}", self.id)).fmt(formatter)
    }
}

pub struct IndexBuffer {
    display: Arc<context::Context>,
    id: gl::types::GLuint,
    elementsCount: uint,
    dataType: gl::types::GLenum,
    primitives: gl::types::GLenum
}

impl fmt::Show for IndexBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
        (format!("IndexBuffer #{} (elements: {})", self.id, self.elementsCount)).fmt(formatter)
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
    fn build_simple_gl(self) -> Result<Display, ()>;
}

impl DisplayBuild for gl_init::WindowBuilder {
    fn build_simple_gl(self) -> Result<Display, ()> {
        let window = try!(self.build().map_err(|_| ()));
        let context = context::Context::new(window);
        Ok(Display { context: Arc::new(context) })
    }
}

impl Display {
    /// Reads all events received by the window.
    pub fn poll_events(&self) -> Vec<gl_init::Event> {
        self.context.recv()
    }

    /// Call this function when you have finished drawing a frame.
    pub fn end_frame(&self) {
        self.context.swap_buffers();

        self.context.exec(proc(gl) {
            gl.ClearColor(0.0, 0.0, 0.0, 1.0);
            gl.Clear(gl::COLOR_BUFFER_BIT);
        });
    }

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
    /// let vertex_buffer = display.build_vertex_buffer(vec![
    ///     Vertex { position: [0.0,  0.0, 0.0], texcoords: [0.0, 1.0] },
    ///     Vertex { position: [5.0, -3.0, 2.0], texcoords: [1.0, 0.0] },
    /// ]);
    /// # }
    /// ```
    /// 
    pub fn build_vertex_buffer<T: VertexFormat + 'static + Send>(&self, data: Vec<T>)
        -> VertexBuffer
    {
        let bindings = VertexFormat::build_bindings(None::<T>);

        let elements_size = { use std::mem; mem::size_of::<T>() };
        let buffer_size = data.len() * elements_size as uint;

        let id = self.context.exec(proc(gl) {
            unsafe {
                let mut id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenBuffers(1, &mut id);
                gl.BindBuffer(gl::ARRAY_BUFFER, id);
                gl.BufferData(gl::ARRAY_BUFFER, buffer_size as gl::types::GLsizeiptr, data.as_ptr() as *const libc::c_void, gl::STATIC_DRAW);
                id
            }
        }).get();

        VertexBuffer {
            display: self.context.clone(),
            id: id,
            elements_size: elements_size,
            bindings: bindings
        }
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
        let elementsSize = std::mem::size_of_val(&data[0]);
        let dataSize = data.len() * elementsSize;
        let dataPtr: *const libc::c_void = data.as_ptr() as *const libc::c_void;

        let id = self.context.exec(proc(gl) {
            unsafe {
                let id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenBuffers(1, std::mem::transmute(&id));
                gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, id);
                gl.BufferData(gl::ELEMENT_ARRAY_BUFFER, dataSize as gl::types::GLsizeiptr, dataPtr, gl::STATIC_DRAW);
                id
            }
        }).get();

        IndexBuffer {
            display: self.context.clone(),
            id: id,
            elementsCount: data.len(),
            dataType: data_types::GLDataType::get_gl_type(None::<T>),
            primitives: prim.get_gl_enum()
        }
    }

    /// Builds an individual shader.
    fn build_shader(&self, stype: gl::types::GLenum, sourceCode: &str)
        -> Result<Arc<ShaderImpl>, String>
    {
        let srcCode = sourceCode.to_string();

        let idResult = self.context.exec(proc(gl) {
            unsafe {
                let id = gl.CreateShader(stype);

                gl.ShaderSource(id, 1, [ srcCode.to_c_str().unwrap() ].as_ptr(), std::ptr::null());
                gl.CompileShader(id);

                let mut compilationSuccess: gl::types::GLint = std::mem::uninitialized();
                gl.GetShaderiv(id, gl::COMPILE_STATUS, &mut compilationSuccess);

                if compilationSuccess == 0 {
                    let mut errorLogSize: gl::types::GLint = std::mem::uninitialized();
                    gl.GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut errorLogSize);

                    let mut errorLog: Vec<u8> = Vec::with_capacity(errorLogSize as uint);
                    gl.GetShaderInfoLog(id, errorLogSize, &mut errorLogSize, errorLog.as_mut_slice().as_mut_ptr() as *mut gl::types::GLchar);

                    let msg = String::from_utf8(errorLog).unwrap();
                    return Err(msg)
                }

                Ok(id)
            }
        }).get();

        idResult.map(|id| {
            Arc::new(ShaderImpl {
                display: self.context.clone(),
                id: id
            })
        })
    }

    /// Builds a new texture.
    pub fn build_texture<T: data_types::GLDataTuple>(&self, data: &[T], width: uint, height: uint, depth: uint, arraySize: uint)
        -> Texture
    {
        let element_components = data_types::GLDataTuple::get_num_elems(None::<T>);

        if width * height * depth * arraySize != data.len() {
            fail!("Texture data has different size from width*height*depth*arraySize*elemLen");
        }

        let textureType = if height == 1 && depth == 1 {
            if arraySize == 1 { gl::TEXTURE_1D } else { gl::TEXTURE_1D_ARRAY }
        } else if depth == 1 {
            if arraySize == 1 { gl::TEXTURE_2D } else { gl::TEXTURE_2D_ARRAY }
        } else {
            gl::TEXTURE_3D
        };

        let data_type = data_types::GLDataTuple::get_gl_type(None::<T>);
        let dataRaw: *const libc::c_void = unsafe { std::mem::transmute(data.as_ptr()) };

        let (data_format, data_type) = match (element_components, data_type) {
            (1, f) => (gl::RED, f),
            (2, f) => (gl::RG, f),
            (3, f) => (gl::RGB, f),
            (4, f) => (gl::RGBA, f),
            _ => fail!("unsupported texture type")
        };

        let id = self.context.exec(proc(gl) {
            unsafe {
                gl.PixelStorei(gl::UNPACK_ALIGNMENT, if width % 4 == 0 { 4 } else if height % 2 == 0 { 2 } else { 1 });

                let id: gl::types::GLuint = std::mem::uninitialized();
                gl.GenTextures(1, std::mem::transmute(&id));

                gl.BindTexture(textureType, id);

                gl.TexParameteri(textureType, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
                if height != 1 || depth != 1 || arraySize != 1 {
                    gl.TexParameteri(textureType, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
                }
                if depth != 1 || arraySize != 1 {
                    gl.TexParameteri(textureType, gl::TEXTURE_WRAP_R, gl::REPEAT as i32);
                }
                gl.TexParameteri(textureType, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                gl.TexParameteri(textureType, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as i32);

                if textureType == gl::TEXTURE_3D || textureType == gl::TEXTURE_2D_ARRAY {
                    gl.TexImage3D(textureType, 0, gl::RGBA as i32, width as i32, height as i32, if depth > 1 { depth } else { arraySize } as i32, 0, data_format as u32, data_type, dataRaw);
                } else if textureType == gl::TEXTURE_2D || textureType == gl::TEXTURE_1D_ARRAY {
                    gl.TexImage2D(textureType, 0, gl::RGBA as i32, width as i32, height as i32, 0, data_format as u32, data_type, dataRaw);
                } else {
                    gl.TexImage1D(textureType, 0, gl::RGBA as i32, width as i32, 0, data_format as u32, data_type, dataRaw);
                }

                gl.GenerateMipmap(textureType);

                id
            }
        }).get();

        Texture {
            texture: Arc::new(TextureImpl {
                display: self.context.clone(),
                id: id,
                bindPoint: textureType,
                width: width,
                height: height,
                depth: depth,
                arraySize: arraySize
            })
        }
    }

    /// Builds a new program.
    ///
    /// A program is a group of shaders linked together.
    ///
    /// # Parameters
    ///
    /// - `vertex_shader`: Source code of the vertex shader.
    /// - `fragment_shader`: Source code of the fragment shader.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
    /// # let vertex_source = ""; let fragment_source = "";
    /// let program = display.build_program(vertex_source, fragment_source);
    /// ```
    /// 
    pub fn build_program(&self, vertex_shader: &str, fragment_shader: &str)
        -> Result<Program, String>
    {
        let mut shadersStore = Vec::new();
        shadersStore.push(try!(self.build_shader(gl::VERTEX_SHADER, vertex_shader)));
        shadersStore.push(try!(self.build_shader(gl::FRAGMENT_SHADER, fragment_shader)));

        let mut shadersIDs = Vec::new();
        for sh in shadersStore.iter() {
            shadersIDs.push(sh.id);
        }

        let id = try!(self.context.exec(proc(gl) {
            unsafe {
                let id = gl.CreateProgram();
                if id == 0 {
                    return Err(format!("glCreateProgram failed"));
                }

                // attaching shaders
                for sh in shadersIDs.iter() {
                    gl.AttachShader(id, sh.clone());
                }

                // linking and checking for errors
                gl.LinkProgram(id);
                {   let mut linkSuccess: gl::types::GLint = std::mem::uninitialized();
                    gl.GetProgramiv(id, gl::LINK_STATUS, &mut linkSuccess);
                    if linkSuccess == 0 {
                        match gl.GetError() {
                            gl::NO_ERROR => (),
                            gl::INVALID_VALUE => return Err(format!("glLinkProgram triggered GL_INVALID_VALUE")),
                            gl::INVALID_OPERATION => return Err(format!("glLinkProgram triggered GL_INVALID_OPERATION")),
                            _ => return Err(format!("glLinkProgram triggered an unknown error"))
                        };

                        let mut errorLogSize: gl::types::GLint = std::mem::uninitialized();
                        gl.GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut errorLogSize);

                        let mut errorLog: Vec<u8> = Vec::with_capacity(errorLogSize as uint);
                        gl.GetProgramInfoLog(id, errorLogSize, &mut errorLogSize, errorLog.as_mut_slice().as_mut_ptr() as *mut gl::types::GLchar);
                        errorLog.set_len(errorLogSize as uint);

                        let msg = String::from_utf8(errorLog).unwrap();
                        return Err(msg)
                    }
                }

                Ok(id)
            }
        }).get());

        let uniforms = self.context.exec(proc(gl) {
            unsafe {
                // reflecting program uniforms
                let mut uniforms = HashMap::new();

                let mut activeUniforms: gl::types::GLint = std::mem::uninitialized();
                gl.GetProgramiv(id, gl::ACTIVE_UNIFORMS, &mut activeUniforms);

                for uniformID in range(0, activeUniforms) {
                    let mut uniformNameTmp: Vec<u8> = Vec::with_capacity(64);
                    let mut uniformNameTmpLen = 63;

                    let mut dataType: gl::types::GLenum = std::mem::uninitialized();
                    let mut dataSize: gl::types::GLint = std::mem::uninitialized();
                    gl.GetActiveUniform(id, uniformID as gl::types::GLuint, uniformNameTmpLen, &mut uniformNameTmpLen, &mut dataSize, &mut dataType, uniformNameTmp.as_mut_slice().as_mut_ptr() as *mut gl::types::GLchar);
                    uniformNameTmp.set_len(uniformNameTmpLen as uint);

                    let uniformName = String::from_utf8(uniformNameTmp).unwrap();
                    let location = gl.GetUniformLocation(id, uniformName.to_c_str().unwrap());

                    uniforms.insert(uniformName, (location, dataType, dataSize));
                }

                Arc::new(uniforms)
            }
        }).get();


        Ok(Program {
            program: Arc::new(ProgramImpl {
                display: self.context.clone(),
                shaders: shadersStore,
                id: id,
                uniforms: uniforms
            })
        })
    }

    /// Draws.
    pub fn draw(&self, vertexBuffer: &VertexBuffer, indexBuffer: &IndexBuffer,
                program: &ProgramUniforms)
    {
        let vbID = vertexBuffer.id.clone();
        let vbBindingsClone = vertexBuffer.bindings.clone();
        let vbElementsSize = vertexBuffer.elements_size.clone();
        let ibID = indexBuffer.id.clone();
        let ibPrimitives = indexBuffer.primitives.clone();
        let ibElemCounts = indexBuffer.elementsCount.clone();
        let ibDataType = indexBuffer.dataType.clone();
        let programID = program.program.id.clone();
        let uniformsClone = program.clone();

        self.context.exec(proc(gl) {
            unsafe {
                gl.Disable(gl::DEPTH_TEST);
                gl.Enable(gl::BLEND);
                gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

                // binding program
                gl.UseProgram(programID);

                // binding program uniforms
                {
                    let mut activeTexture: uint = 0;
                    for (&location, ref texture) in uniformsClone.textures.iter() {
                        gl.ActiveTexture(gl::TEXTURE0 + activeTexture as u32);
                        gl.BindTexture(texture.bindPoint, texture.id);
                        gl.Uniform1i(location, activeTexture as i32);
                        activeTexture = activeTexture + 1;
                    }

                    for (&location, &(ref datatype, ref data)) in uniformsClone.values.iter() {
                        match *datatype {
                            gl::FLOAT       => gl.Uniform1fv(location, 1, data.as_ptr() as *const f32),
                            gl::FLOAT_MAT4  => gl.UniformMatrix4fv(location, 1, 0, data.as_ptr() as *const f32),
                            _ => fail!("Loading uniforms for this type not implemented")
                        }
                        //gl.Uniform1i(location, activeTexture as i32);
                    }
                }

                // binding buffers
                gl.BindBuffer(gl::ARRAY_BUFFER, vbID);
                gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibID);

                // binding vertex buffer
                let mut locations = Vec::new();
                for (name, &(dataType, dataSize, dataOffset)) in vbBindingsClone.iter() {
                    let loc = gl.GetAttribLocation(programID, name.to_c_str().unwrap());
                    locations.push(loc);

                    if loc != -1 {
                        match dataType {
                            gl::BYTE | gl::UNSIGNED_BYTE | gl::SHORT | gl::UNSIGNED_SHORT | gl::INT | gl::UNSIGNED_INT
                                => gl.VertexAttribIPointer(loc as u32, dataSize, dataType, vbElementsSize as i32, dataOffset as *const libc::c_void),
                            _ => gl.VertexAttribPointer(loc as u32, dataSize, dataType, 0, vbElementsSize as i32, dataOffset as *const libc::c_void)
                        }
                        
                        gl.EnableVertexAttribArray(loc as u32);
                    }
                }
                
                // drawing
                gl.DrawElements(ibPrimitives, ibElemCounts as i32, ibDataType, std::ptr::null());

                // disable vertex attrib array
                for l in locations.iter() {
                    gl.DisableVertexAttribArray(l.clone() as u32);
                }
            }
        }).get();
    }
}

impl PrimitiveType {
    fn get_gl_enum(&self) -> gl::types::GLenum {
        match *self {
            PointsList => gl::POINTS,
            LinesList => gl::LINES,
            LineStrip => gl::LINE_STRIP,
            TrianglesList => gl::TRIANGLES,
            TrianglesListAdjacency => gl::TRIANGLES_ADJACENCY,
            TriangleStrip => gl::TRIANGLE_STRIP,
            TriangleStripAdjacency => gl::TRIANGLE_STRIP_ADJACENCY,
            TriangleFan => gl::TRIANGLE_FAN
        }
    }
}

impl Texture {
    pub fn get_width(&self) -> uint {
        self.texture.width
    }

    pub fn get_height(&self) -> uint {
        self.texture.height
    }

    pub fn get_depth(&self) -> uint {
        self.texture.depth
    }

    pub fn get_array_size(&self) -> uint {
        self.texture.arraySize
    }
}

impl Program {
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

        let dataInside = data.as_mut_ptr() as *mut T;
        unsafe { (*dataInside) = value; }

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

impl Drop for TextureImpl {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.exec(proc(gl) {
            unsafe { gl.DeleteTextures(1, [ id ].as_ptr()); }
        });
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.exec(proc(gl) {
            unsafe { gl.DeleteBuffers(1, [ id ].as_ptr()); }
        });
    }
}

impl Drop for IndexBuffer {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.exec(proc(gl) {
            unsafe { gl.DeleteBuffers(1, [ id ].as_ptr()); }
        });
    }
}

impl Drop for ShaderImpl {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.exec(proc(gl) {
            gl.DeleteShader(id);
        });
    }
}
