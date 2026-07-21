// src/main.rs
mod camera;
mod mesh;
mod physics;
mod shader;

use camera::Camera;
use mesh::Mesh;
use physics::{PhysicsWorld, RigidBody};

use glam::{Mat4, Quat, Vec3};
use glfw::{Action, Context, Key, MouseButton, WindowEvent};
use std::fs;
use std::path::Path;

const VERTEX_SHADER_SRC: &str = r#"
#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec2 aTexCoord;

uniform mat4 u_Model;
uniform mat4 u_View;
uniform mat4 u_Projection;

out vec3 Normal;
out vec3 FragPos;

void main() {
    FragPos = vec3(u_Model * vec4(aPos, 1.0));
    Normal = mat3(transpose(inverse(u_Model))) * aNormal;
    gl_Position = u_Projection * u_View * vec4(FragPos, 1.0);
}
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
#version 330 core
out vec4 FragColor;

in vec3 Normal;
in vec3 FragPos;

uniform vec3 u_Color;

void main() {
    vec3 lightDir = normalize(vec3(0.4, 0.9, 0.5));
    float diff = max(dot(normalize(Normal), lightDir), 0.3);
    vec3 result = u_Color * diff;
    FragColor = vec4(result, 1.0);
}
"#;

pub struct PropTemplate {
    pub name: String,
    pub mesh: Mesh,
    pub collider_size: Vec3,
}

pub struct RenderableProp {
    pub body_id: usize,
    pub template_index: usize,
}

fn ray_aabb_intersect(origin: Vec3, dir: Vec3, box_min: Vec3, box_max: Vec3) -> Option<f32> {
    let mut tmin = (box_min.x - origin.x) / dir.x;
    let mut tmax = (box_max.x - origin.x) / dir.x;
    if tmin > tmax { std::mem::swap(&mut tmin, &mut tmax); }

    let mut tymin = (box_min.y - origin.y) / dir.y;
    let mut tymax = (box_max.y - origin.y) / dir.y;
    if tymin > tymax { std::mem::swap(&mut tymin, &mut tymax); }

    if (tmin > tymax) || (tymin > tmax) { return None; }

    if tymin > tmin { tmin = tymin; }
    if tymax < tmax { tmax = tymax; }

    let mut tzmin = (box_min.z - origin.z) / dir.z;
    let mut tzmax = (box_max.z - origin.z) / dir.z;
    if tzmin > tzmax { std::mem::swap(&mut tzmin, &mut tzmax); }

    if (tmin > tzmax) || (tzmin > tmax) { return None; }

    if tzmin > tmin { tmin = tzmin; }

    if tmin < 0.0 { return None; }

    Some(tmin)
}

fn create_embedded_shader() -> u32 {
    unsafe {
        let compile = |src: &str, shader_type: u32| {
            let shader = gl::CreateShader(shader_type);
            let c_str = std::ffi::CString::new(src.as_bytes()).unwrap();
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut success = gl::FALSE as i32;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = vec![0u8; len as usize];
                gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
                panic!(" Ошибка компиляции шейдера: {}", String::from_utf8_lossy(&buffer));
            }
            shader
        };

        let vs = compile(VERTEX_SHADER_SRC, gl::VERTEX_SHADER);
        let fs = compile(FRAGMENT_SHADER_SRC, gl::FRAGMENT_SHADER);

        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);

        program
    }
}

fn create_cube_mesh() -> Mesh {
    #[rustfmt::skip]
    let vertices: [f32; 288] = [
        -0.5, -0.5, -0.5,  0.0,  0.0, -1.0,  0.0, 0.0,
         0.5, -0.5, -0.5,  0.0,  0.0, -1.0,  1.0, 0.0,
         0.5,  0.5, -0.5,  0.0,  0.0, -1.0,  1.0, 1.0,
         0.5,  0.5, -0.5,  0.0,  0.0, -1.0,  1.0, 1.0,
        -0.5,  0.5, -0.5,  0.0,  0.0, -1.0,  0.0, 1.0,
        -0.5, -0.5, -0.5,  0.0,  0.0, -1.0,  0.0, 0.0,

        -0.5, -0.5,  0.5,  0.0,  0.0,  1.0,  0.0, 0.0,
         0.5, -0.5,  0.5,  0.0,  0.0,  1.0,  1.0, 0.0,
         0.5,  0.5,  0.5,  0.0,  0.0,  1.0,  1.0, 1.0,
         0.5,  0.5,  0.5,  0.0,  0.0,  1.0,  1.0, 1.0,
        -0.5,  0.5,  0.5,  0.0,  0.0,  1.0,  0.0, 1.0,
        -0.5, -0.5,  0.5,  0.0,  0.0,  1.0,  0.0, 0.0,

        -0.5,  0.5,  0.5, -1.0,  0.0,  0.0,  1.0, 0.0,
        -0.5,  0.5, -0.5, -1.0,  0.0,  0.0,  1.0, 1.0,
        -0.5, -0.5, -0.5, -1.0,  0.0,  0.0,  0.0, 1.0,
        -0.5, -0.5, -0.5, -1.0,  0.0,  0.0,  0.0, 1.0,
        -0.5, -0.5,  0.5, -1.0,  0.0,  0.0,  0.0, 0.0,
        -0.5,  0.5,  0.5, -1.0,  0.0,  0.0,  1.0, 0.0,

         0.5,  0.5,  0.5,  1.0,  0.0,  0.0,  1.0, 0.0,
         0.5,  0.5, -0.5,  1.0,  0.0,  0.0,  1.0, 1.0,
         0.5, -0.5, -0.5,  1.0,  0.0,  0.0,  0.0, 1.0,
         0.5, -0.5, -0.5,  1.0,  0.0,  0.0,  0.0, 1.0,
         0.5, -0.5,  0.5,  1.0,  0.0,  0.0,  0.0, 0.0,
         0.5,  0.5,  0.5,  1.0,  0.0,  0.0,  1.0, 0.0,

        -0.5, -0.5, -0.5,  0.0, -1.0,  0.0,  0.0, 1.0,
         0.5, -0.5, -0.5,  0.0, -1.0,  0.0,  1.0, 1.0,
         0.5, -0.5,  0.5,  0.0, -1.0,  0.0,  1.0, 0.0,
         0.5, -0.5,  0.5,  0.0, -1.0,  0.0,  1.0, 0.0,
        -0.5, -0.5,  0.5,  0.0, -1.0,  0.0,  0.0, 0.0,
        -0.5, -0.5, -0.5,  0.0, -1.0,  0.0,  0.0, 1.0,

        -0.5,  0.5, -0.5,  0.0,  1.0,  0.0,  0.0, 1.0,
         0.5,  0.5, -0.5,  0.0,  1.0,  0.0,  1.0, 1.0,
         0.5,  0.5,  0.5,  0.0,  1.0,  0.0,  1.0, 0.0,
         0.5,  0.5,  0.5,  0.0,  1.0,  0.0,  1.0, 0.0,
        -0.5,  0.5,  0.5,  0.0,  1.0,  0.0,  0.0, 0.0,
        -0.5,  0.5, -0.5,  0.0,  1.0,  0.0,  0.0, 1.0,
    ];

    Mesh::new_interleaved(&vertices, 36)
}

fn set_shader_mat4(program_id: u32, name: &str, mat: &Mat4) {
    let c_name = std::ffi::CString::new(name).unwrap();
    unsafe {
        let loc = gl::GetUniformLocation(program_id, c_name.as_ptr());
        if loc != -1 {
            gl::UniformMatrix4fv(loc, 1, gl::FALSE, mat.to_cols_array().as_ptr());
        }
    }
}

fn set_shader_vec3(program_id: u32, name: &str, vec: Vec3) {
    let c_name = std::ffi::CString::new(name).unwrap();
    unsafe {
        let loc = gl::GetUniformLocation(program_id, c_name.as_ptr());
        if loc != -1 {
            gl::Uniform3f(loc, vec.x, vec.y, vec.z);
        }
    }
}

fn ensure_props_dir() -> Vec<PropTemplate> {
    let props_dir = Path::new("props");
    if !props_dir.exists() {
        let _ = fs::create_dir_all(props_dir);

        let sample_obj = r#"
# Sample Pyramid Prop
v -0.8 0.0 -0.8
v  0.8 0.0 -0.8
v  0.8 0.0  0.8
v -0.8 0.0  0.8
v  0.0 1.6  0.0
vn 0 1 0
f 1 2 5
f 2 3 5
f 3 4 5
f 4 1 5
"#;
        let _ = fs::write(props_dir.join("sample_pyramid.obj"), sample_obj);
    }

    let mut templates = Vec::new();

    if let Ok(entries) = fs::read_dir(props_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("obj") {
                let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
                println!("[PropsManager] Загрузка модели: {:?}", path);

                match Mesh::load_obj(&path) {
                    Ok((mesh, size)) => {
                        println!("  -> Авто-размер AABB: {:?}", size);
                        templates.push(PropTemplate {
                            name: file_name,
                            mesh,
                            collider_size: size,
                        });
                    }
                    Err(e) => {
                        eprintln!("  -> Ошибка загрузки {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    templates
}

fn main() {
    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

    let (mut window, events) = glfw
        .create_window(1280, 720, "TEngine - GravGun & Hitboxes", glfw::WindowMode::Windowed)
        .expect("Не удалось создать окно GLFW.");

    window.make_current();
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_mouse_button_polling(true);
    window.set_scroll_polling(true);
    window.set_cursor_mode(glfw::CursorMode::Disabled);

    gl::load_with(|s| window.get_proc_address(s) as *const _);

    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Disable(gl::CULL_FACE);
    }

    let shader_program = create_embedded_shader();

    let cube_mesh = create_cube_mesh();
    let ground_size = Vec3::new(40.0, 1.0, 40.0);
    let ground_pos = Vec3::new(0.0, -0.5, 0.0);

    let prop_templates = ensure_props_dir();

    let mut physics = PhysicsWorld::new();

    // Земля (ID = 0)
    let ground_body = RigidBody::new(ground_pos, ground_size, 0.0, true);
    let ground_id = physics.add_body(ground_body);

    // Игрок (ID = 1)
    let player_body = RigidBody::new(Vec3::new(0.0, 2.0, 6.0), Vec3::new(0.8, 1.8, 0.8), 70.0, false);
    let player_id = physics.add_body(player_body);

    let mut camera = Camera::new(Vec3::new(0.0, 2.0, 6.0));
    let mut renderable_props: Vec<RenderableProp> = Vec::new();
    let mut selected_template_idx = 0;

    // ФЛАГ ОТОБРАЖЕНИЯ ХИТБОКСОВ
    let mut show_hitboxes = false;

    // Грави-пушка
    let mut held_body_id: Option<usize> = None;
    let mut hold_distance: f32 = 4.0;

    // Спавн 1 пропа при старте
    if !prop_templates.is_empty() {
        let tmpl = &prop_templates[0];
        let spawn_pos = Vec3::new(0.0, 3.0, 0.0);
        let prop_body = RigidBody::new(spawn_pos, tmpl.collider_size, 15.0, false);
        let body_id = physics.add_body(prop_body);

        renderable_props.push(RenderableProp {
            body_id,
            template_index: 0,
        });
    }

    let mut first_mouse = true;
    let mut last_x = 1280.0 / 2.0;
    let mut last_y = 720.0 / 2.0;

    let mut last_frame = glfw.get_time();

    while !window.should_close() {
        let current_frame = glfw.get_time();
        let delta_time = (current_frame - last_frame) as f32;
        last_frame = current_frame;

        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }

                // ПЕРЕКЛЮЧЕНИЕ ХИТБОКСОВ НА CTRL + H
                WindowEvent::Key(Key::H, _, Action::Press, _) => {
                    if window.get_key(Key::LeftControl) == Action::Press || window.get_key(Key::RightControl) == Action::Press {
                        show_hitboxes = !show_hitboxes;
                        println!("[Debug] Отображение хитбоксов: {}", show_hitboxes);
                    }
                }

                WindowEvent::CursorPos(xpos, ypos) => {
                    let xpos = xpos as f32;
                    let ypos = ypos as f32;

                    if first_mouse {
                        last_x = xpos;
                        last_y = ypos;
                        first_mouse = false;
                    }

                    let xoffset = xpos - last_x;
                    let yoffset = last_y - ypos;
                    last_x = xpos;
                    last_y = ypos;

                    camera.process_mouse_movement(xoffset, yoffset, true);
                }
                WindowEvent::Scroll(_, yoffset) => {
                    if held_body_id.is_some() {
                        hold_distance = (hold_distance + (yoffset as f32) * 0.5).clamp(2.0, 10.0);
                    }
                }
                WindowEvent::MouseButton(button, action, _) => {
                    if button == MouseButton::Button1 {
                        if action == Action::Press {
                            let mut closest_t = 15.0;
                            let mut target_id = None;

                            for (id, body) in physics.bodies.iter().enumerate() {
                                if id == ground_id || id == player_id { continue; }

                                // Получаем физический размер тела
                                let body_size = body.size();
                                let box_min = body.position - body_size * 0.5;
                                let box_max = body.position + body_size * 0.5;

                                if let Some(t) = ray_aabb_intersect(camera.position, camera.front, box_min, box_max) {
                                    if t < closest_t {
                                        closest_t = t;
                                        target_id = Some(id);
                                    }
                                }
                            }

                            if let Some(id) = target_id {
                                held_body_id = Some(id);
                                hold_distance = closest_t.clamp(2.0, 8.0);
                            }
                        } else if action == Action::Release {
                            held_body_id = None;
                        }
                    } else if button == MouseButton::Button2 && action == Action::Press {
                        if let Some(id) = held_body_id.take() {
                            if let Some(body) = physics.get_body_mut(id) {
                                body.velocity = camera.front * 32.0;
                            }
                        }
                    }
                }
                WindowEvent::Key(Key::E, _, Action::Press, _) => {
                    if !prop_templates.is_empty() {
                        let tmpl = &prop_templates[selected_template_idx];
                        let spawn_pos = camera.position + camera.front * 3.0;

                        let prop_body = RigidBody::new(spawn_pos, tmpl.collider_size, 15.0, false);
                        let body_id = physics.add_body(prop_body);

                        renderable_props.push(RenderableProp {
                            body_id,
                            template_index: selected_template_idx,
                        });

                        selected_template_idx = (selected_template_idx + 1) % prop_templates.len();
                    }
                }
                _ => {}
            }
        }

        // УПРАВЛЕНИЕ ИГРОКОМ WASD
        let move_speed = 7.0;
        let mut move_dir = Vec3::ZERO;

        let mut forward = camera.front;
        forward.y = 0.0;
        if forward.length_squared() > 0.0 { forward = forward.normalize(); }

        let mut right = camera.right;
        right.y = 0.0;
        if right.length_squared() > 0.0 { right = right.normalize(); }

        if window.get_key(Key::W) == Action::Press { move_dir += forward; }
        if window.get_key(Key::S) == Action::Press { move_dir -= forward; }
        if window.get_key(Key::A) == Action::Press { move_dir -= right; }
        if window.get_key(Key::D) == Action::Press { move_dir += right; }

        if move_dir.length_squared() > 0.0 { move_dir = move_dir.normalize(); }

        if let Some(p_body) = physics.get_body_mut(player_id) {
            p_body.velocity.x = move_dir.x * move_speed;
            p_body.velocity.z = move_dir.z * move_speed;

            if window.get_key(Key::Space) == Action::Press && p_body.is_grounded {
                p_body.velocity.y = 6.0;
            }
        }

        // ГРАВИ-ПУШКА
        if let Some(id) = held_body_id {
            let target_pos = camera.position + camera.front * hold_distance;
            if let Some(body) = physics.get_body_mut(id) {
                let to_target = target_pos - body.position;
                body.velocity = to_target * 16.0;
            }
        }

        physics.step(delta_time);

        if let Some(p_body) = physics.get_body(player_id) {
            camera.position = p_body.position + Vec3::new(0.0, 0.6, 0.0);
        }

        // РЕНДЕРИНГ ОСНОВНОЙ СЦЕНЫ
        unsafe {
            gl::ClearColor(0.2, 0.35, 0.5, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::UseProgram(shader_program);
        }

        let projection = Mat4::perspective_rh_gl(45.0f32.to_radians(), 1280.0 / 720.0, 0.1, 100.0);
        let view = camera.get_view_matrix();

        set_shader_mat4(shader_program, "u_Projection", &projection);
        set_shader_mat4(shader_program, "u_View", &view);

        // 1. Земля
        let ground_model = Mat4::from_scale_rotation_translation(ground_size, Quat::IDENTITY, ground_pos);
        set_shader_mat4(shader_program, "u_Model", &ground_model);
        set_shader_vec3(shader_program, "u_Color", Vec3::new(0.25, 0.6, 0.25));
        cube_mesh.draw();

        // 2. Пропы
        for prop in &renderable_props {
            if let Some(body) = physics.get_body(prop.body_id) {
                let tmpl = &prop_templates[prop.template_index];

                let model = Mat4::from_translation(body.position);
                set_shader_mat4(shader_program, "u_Model", &model);

                let color = if Some(prop.body_id) == held_body_id {
                    Vec3::new(1.0, 0.7, 0.1)
                } else {
                    Vec3::new(0.85, 0.45, 0.15)
                };

                set_shader_vec3(shader_program, "u_Color", color);
                tmpl.mesh.draw();
            }
        }

        // 3. ОТЛАДОЧНЫЙ РЕНДЕРИНГ ХИТБОКСОВ (CTRL + H)
        if show_hitboxes {
            unsafe {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE); // Режим каркаса
                gl::Disable(gl::DEPTH_TEST);                  // X-Ray эффект (виден сквозь геометрию)
            }

            for (id, body) in physics.bodies.iter().enumerate() {
                let size = body.size();
                let model = Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, body.position);

                set_shader_mat4(shader_program, "u_Model", &model);

                let hitbox_color = if id == ground_id {
                    Vec3::new(0.0, 1.0, 0.8) // Голубой хитбокс Земли
                } else if id == player_id {
                    Vec3::new(1.0, 0.2, 0.2) // Красный хитбокс Игрока
                } else if Some(id) == held_body_id {
                    Vec3::new(1.0, 1.0, 0.0) // Желтый хитбокс захваченного объекта
                } else {
                    Vec3::new(0.0, 1.0, 0.0) // Зеленые хитбоксы всех пропов
                };

                set_shader_vec3(shader_program, "u_Color", hitbox_color);
                cube_mesh.draw();
            }

            unsafe {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL); // Возвращаем нормальный режим
                gl::Enable(gl::DEPTH_TEST);
            }
        }

        window.swap_buffers();
    }
}