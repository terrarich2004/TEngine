use gl::types::*;
use glam::{Mat4, Vec3};
use std::ffi::CString;
use std::fs;
use std::ptr;
use std::str;

pub struct Shader {
    pub id: GLuint,
}

// Встроенный вертекстный шейдер по умолчанию (если файл на диске не найден)
const DEFAULT_VERTEX_SHADER: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;

out vec3 FragPos;
out vec3 Normal;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main() {
    FragPos = vec3(model * vec4(aPos, 1.0));
    Normal = mat3(transpose(inverse(model))) * aNormal;
    gl_Position = projection * view * vec4(FragPos, 1.0);
}
"#;

// Встроенный фрагментный шейдер по умолчанию
const DEFAULT_FRAGMENT_SHADER: &str = r#"
#version 330 core
out vec4 FragColor;

in vec3 FragPos;
in vec3 Normal;

uniform vec3 objectColor;

void main() {
    vec3 norm = normalize(Normal);
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));
    float diff = max(dot(norm, lightDir), 0.2);
    vec3 result = objectColor * diff;
    FragColor = vec4(result, 1.0);
}
"#;

impl Shader {
    pub fn new(vertex_path: &str, fragment_path: &str) -> Self {
        let vertex_code = fs::read_to_string(vertex_path)
            .unwrap_or_else(|_| DEFAULT_VERTEX_SHADER.to_string());
        let fragment_code = fs::read_to_string(fragment_path)
            .unwrap_or_else(|_| DEFAULT_FRAGMENT_SHADER.to_string());

        let c_vertex_code = CString::new(vertex_code.as_bytes()).unwrap();
        let c_fragment_code = CString::new(fragment_code.as_bytes()).unwrap();

        unsafe {
            let vertex = gl::CreateShader(gl::VERTEX_SHADER);
            gl::ShaderSource(vertex, 1, &c_vertex_code.as_ptr(), ptr::null());
            gl::CompileShader(vertex);
            Self::check_compile_errors(vertex, "VERTEX");

            let fragment = gl::CreateShader(gl::FRAGMENT_SHADER);
            gl::ShaderSource(fragment, 1, &c_fragment_code.as_ptr(), ptr::null());
            gl::CompileShader(fragment);
            Self::check_compile_errors(fragment, "FRAGMENT");

            let id = gl::CreateProgram();
            gl::AttachShader(id, vertex);
            gl::AttachShader(id, fragment);
            gl::LinkProgram(id);
            Self::check_compile_errors(id, "PROGRAM");

            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);

            Self { id }
        }
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_mat4(&self, name: &str, mat: &Mat4) {
        let c_name = CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            gl::UniformMatrix4fv(location, 1, gl::FALSE, mat.to_cols_array().as_ptr());
        }
    }

    pub fn set_vec3(&self, name: &str, value: Vec3) {
        let c_name = CString::new(name).unwrap();
        unsafe {
            let location = gl::GetUniformLocation(self.id, c_name.as_ptr());
            gl::Uniform3f(location, value.x, value.y, value.z);
        }
    }

    unsafe fn check_compile_errors(shader: GLuint, shader_type: &str) {
        unsafe {
            let mut success: GLint = 0;
            let mut info_log = vec![0u8; 1024];

            if shader_type != "PROGRAM" {
                gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
                if success == 0 {
                    gl::GetShaderInfoLog(
                        shader,
                        1024,
                        ptr::null_mut(),
                        info_log.as_mut_ptr() as *mut GLchar,
                    );
                    println!(
                        "ERROR::SHADER_COMPILATION_ERROR of type: {}\n{}",
                        shader_type,
                        str::from_utf8(&info_log).unwrap_or("Unknown UTF-8 error")
                    );
                }
            } else {
                gl::GetProgramiv(shader, gl::LINK_STATUS, &mut success);
                if success == 0 {
                    gl::GetProgramInfoLog(
                        shader,
                        1024,
                        ptr::null_mut(),
                        info_log.as_mut_ptr() as *mut GLchar,
                    );
                    println!(
                        "ERROR::PROGRAM_LINKING_ERROR of type: {}\n{}",
                        shader_type,
                        str::from_utf8(&info_log).unwrap_or("Unknown UTF-8 error")
                    );
                }
            }
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