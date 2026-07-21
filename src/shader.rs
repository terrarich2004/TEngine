use gl::types::*;
use glam::Mat4;
use std::ffi::CString;
use std::ptr;
use std::str;

pub struct Shader {
    pub id: u32,
}

impl Shader {
    pub fn new(vertex_code: &str, fragment_code: &str) -> Self {
        let vs = Self::compile_shader(gl::VERTEX_SHADER, vertex_code);
        let fs = Self::compile_shader(gl::FRAGMENT_SHADER, fragment_code);
        let id = Self::link_program(vs, fs);

        unsafe {
            gl::DeleteShader(vs);
            gl::DeleteShader(fs);
        }

        Self { id }
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_mat4(&self, name: &str, mat: &Mat4) {
        unsafe {
            let c_name = CString::new(name).unwrap();
            let loc = gl::GetUniformLocation(self.id, c_name.as_ptr());
            gl::UniformMatrix4fv(loc, 1, gl::FALSE, mat.to_cols_array().as_ptr());
        }
    }

    fn compile_shader(shader_type: GLenum, source: &str) -> u32 {
        unsafe {
            let shader = gl::CreateShader(shader_type);
            let c_str = CString::new(source.as_bytes()).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader);

            let mut success = gl::FALSE as GLint;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = vec![0u8; len as usize];
                gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
                panic!("Ошибка компиляции шейдера: {}", String::from_utf8_lossy(&buffer));
            }
            shader
        }
    }

    fn link_program(vs: u32, fs: u32) -> u32 {
        unsafe {
            let program = gl::CreateProgram();
            gl::AttachShader(program, vs);
            gl::AttachShader(program, fs);
            gl::LinkProgram(program);

            let mut success = gl::FALSE as GLint;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                let mut len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = vec![0u8; len as usize];
                gl::GetProgramInfoLog(program, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
                panic!("Ошибка линковки программы шейдеров: {}", String::from_utf8_lossy(&buffer));
            }
            program
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}