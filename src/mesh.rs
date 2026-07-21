use gl::types::*;
use std::mem;
use std::ptr;

pub struct Mesh {
    pub vao: GLuint,
    pub vbo: GLuint,
    pub ebo: GLuint,
    pub index_count: i32,
}

impl Mesh {
    pub fn new(vertices: &[f32], indices: &[u32]) -> Self {
        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<f32>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<u32>()) as GLsizeiptr,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            let stride = (6 * mem::size_of::<f32>()) as GLsizei;
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::EnableVertexAttribArray(0);

            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * mem::size_of::<f32>()) as *const std::ffi::c_void,
            );
            gl::EnableVertexAttribArray(1);

            gl::BindVertexArray(0);
        }

        Self {
            vao,
            vbo,
            ebo,
            index_count: indices.len() as i32,
        }
    }

    pub fn create_cube() -> Self {
        #[rustfmt::skip]
        let vertices: [f32; 144] = [
            // Pos (3)           // Normal (3)
            -1.0, -1.0, -1.0,   0.0,  0.0, -1.0,
             1.0, -1.0, -1.0,   0.0,  0.0, -1.0,
             1.0,  1.0, -1.0,   0.0,  0.0, -1.0,
            -1.0,  1.0, -1.0,   0.0,  0.0, -1.0,

            -1.0, -1.0,  1.0,   0.0,  0.0,  1.0,
             1.0, -1.0,  1.0,   0.0,  0.0,  1.0,
             1.0,  1.0,  1.0,   0.0,  0.0,  1.0,
            -1.0,  1.0,  1.0,   0.0,  0.0,  1.0,

            -1.0,  1.0,  1.0,  -1.0,  0.0,  0.0,
            -1.0,  1.0, -1.0,  -1.0,  0.0,  0.0,
            -1.0, -1.0, -1.0,  -1.0,  0.0,  0.0,
            -1.0, -1.0,  1.0,  -1.0,  0.0,  0.0,

             1.0,  1.0,  1.0,   1.0,  0.0,  0.0,
             1.0,  1.0, -1.0,   1.0,  0.0,  0.0,
             1.0, -1.0, -1.0,   1.0,  0.0,  0.0,
             1.0, -1.0,  1.0,   1.0,  0.0,  0.0,

            -1.0, -1.0, -1.0,   0.0, -1.0,  0.0,
             1.0, -1.0, -1.0,   0.0, -1.0,  0.0,
             1.0, -1.0,  1.0,   0.0, -1.0,  0.0,
            -1.0, -1.0,  1.0,   0.0, -1.0,  0.0,

            -1.0,  1.0, -1.0,   0.0,  1.0,  0.0,
             1.0,  1.0, -1.0,   0.0,  1.0,  0.0,
             1.0,  1.0,  1.0,   0.0,  1.0,  0.0,
            -1.0,  1.0,  1.0,   0.0,  1.0,  0.0,
        ];

        #[rustfmt::skip]
        let indices: [u32; 36] = [
             0,  2,  1,  0,  3,  2,
             4,  5,  6,  4,  6,  7,
             8,  9, 10,  8, 10, 11,
            12, 14, 13, 12, 15, 14,
            16, 17, 18, 16, 18, 19,
            20, 22, 21, 20, 23, 22,
        ];

        Self::new(&vertices, &indices)
    }

    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, self.index_count, gl::UNSIGNED_INT, ptr::null());
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
        }
    }
}