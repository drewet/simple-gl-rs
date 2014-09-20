use data_types;
use gl;
use libc;
use std::fmt;
use std::mem;
use std::sync::Arc;

/// A texture usable by OpenGL.
pub struct Texture {
    texture: Arc<TextureImpl>
}

pub fn get_impl<'a>(texture: &'a Texture) -> &'a Arc<TextureImpl> {
    &texture.texture
}

impl Texture {
    /// Builds a new texture.
    pub fn new<T: data_types::GLDataTuple>(display: &super::Display, data: &[T], width: uint, height: uint, depth: uint, array_size: uint)
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
        let data_raw: *const libc::c_void = unsafe { mem::transmute(data.as_ptr()) };

        let (data_format, data_type) = match (element_components, data_type) {
            (1, f) => (gl::RED, f),
            (2, f) => (gl::RG, f),
            (3, f) => (gl::RGB, f),
            (4, f) => (gl::RGBA, f),
            _ => fail!("unsupported texture type")
        };

        let id = display.context.context.exec(proc(gl) {
            unsafe {
                gl.PixelStorei(gl::UNPACK_ALIGNMENT, if width % 4 == 0 { 4 } else if height % 2 == 0 { 2 } else { 1 });

                let id: gl::types::GLuint = mem::uninitialized();
                gl.GenTextures(1, mem::transmute(&id));

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
                display: display.context.clone(),
                id: id,
                bind_point: texture_type,
                width: width,
                height: height,
                depth: depth,
                array_size: array_size
            })
        }
    }

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
    pub fn draw(&mut self) -> super::Target {
        let display = self.texture.display.clone();
        let fbo = super::FrameBufferObject::new(display.clone());

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
        super::Target {
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

pub struct TextureImpl {
    pub display: Arc<super::DisplayImpl>,
    pub id: gl::types::GLuint,
    pub bind_point: gl::types::GLenum,
    pub width: uint,
    pub height: uint,
    pub depth: uint,
    pub array_size: uint
}

impl Drop for TextureImpl {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl) {
            unsafe { gl.DeleteTextures(1, [ id ].as_ptr()); }
        });
    }
}
