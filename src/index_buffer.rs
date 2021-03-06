use data_types;
use gl;
use libc;
use std::fmt;
use std::mem;
use std::sync::Arc;
use PrimitiveType;

/// A list of indices loaded in the graphics card's memory.
pub struct IndexBuffer {
    display: Arc<super::DisplayImpl>,
    id: gl::types::GLuint,
    elements_count: uint,
    data_type: gl::types::GLenum,
    primitives: gl::types::GLenum
}

/// This public function is accessible from within `simple_gl` but not for the user.
pub fn get_clone(ib: &IndexBuffer) -> (gl::types::GLuint, uint, gl::types::GLenum, gl::types::GLenum) {
    (ib.id.clone(), ib.elements_count.clone(), ib.data_type.clone(), ib.primitives.clone())
}

impl IndexBuffer {
    /// Builds a new index buffer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let display: simple_gl::Display = unsafe { std::mem::uninitialized() };
    /// let index_buffer = simple_gl::IndexBuffer::new(&display, simple_gl::TrianglesList,
    ///     &[0u8, 1, 2, 1, 3, 4, 2, 4, 3]);
    /// ```
    /// 
    pub fn new<T: data_types::GLDataType>(display: &super::Display, prim: PrimitiveType, data: &[T]) -> IndexBuffer {
        let elements_size = mem::size_of_val(&data[0]);
        let data_size = data.len() * elements_size;
        let data_ptr: *const libc::c_void = data.as_ptr() as *const libc::c_void;

        let id = display.context.context.exec(proc(gl) {
            unsafe {
                let id: gl::types::GLuint = mem::uninitialized();
                gl.GenBuffers(1, mem::transmute(&id));
                gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, id);
                gl.BufferData(gl::ELEMENT_ARRAY_BUFFER, data_size as gl::types::GLsizeiptr, data_ptr, gl::STATIC_DRAW);
                id
            }
        }).get();

        IndexBuffer {
            display: display.context.clone(),
            id: id,
            elements_count: data.len(),
            data_type: data_types::GLDataType::get_gl_type(None::<T>),
            primitives: prim.get_gl_enum()
        }
    }
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
