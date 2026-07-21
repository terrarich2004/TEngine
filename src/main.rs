mod camera;
mod mesh;
mod physics;
mod shader;

use camera::Camera;
use glfw::{Action, Context, CursorMode, Key, MouseButton, WindowEvent};
use glam::{Mat4, Vec3};
use mesh::Mesh;
use physics::{PhysicsWorld, RigidBody};
use shader::Shader;

const VERTEX_SHADER_SRC: &str = r#"
    #version 330 core
    layout (location = 0) in vec3 aPos;
    layout (location = 1) in vec3 aColor;

    out vec3 OurColor;
    uniform mat4 uMVP;

    void main() {
        gl_Position = uMVP * vec4(aPos, 1.0);
        OurColor = aColor;
    }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
    #version 330 core
    out vec4 FragColor;
    in vec3 OurColor;

    void main() {
        FragColor = vec4(OurColor, 1.0);
    }
"#;

#[rustfmt::skip]
const CUBE_VERTICES: &[f32] = &[
    // Positions        // Colors
    -0.5, -0.5, -0.5,   0.9, 0.3, 0.3,
     0.5, -0.5, -0.5,   0.9, 0.3, 0.3,
     0.5,  0.5, -0.5,   0.9, 0.3, 0.3,
     0.5,  0.5, -0.5,   0.9, 0.3, 0.3,
    -0.5,  0.5, -0.5,   0.9, 0.3, 0.3,
    -0.5, -0.5, -0.5,   0.9, 0.3, 0.3,

    -0.5, -0.5,  0.5,   0.3, 0.9, 0.3,
     0.5, -0.5,  0.5,   0.3, 0.9, 0.3,
     0.5,  0.5,  0.5,   0.3, 0.9, 0.3,
     0.5,  0.5,  0.5,   0.3, 0.9, 0.3,
    -0.5,  0.5,  0.5,   0.3, 0.9, 0.3,
    -0.5, -0.5,  0.5,   0.3, 0.9, 0.3,

    -0.5,  0.5,  0.5,   0.3, 0.3, 0.9,
    -0.5,  0.5, -0.5,   0.3, 0.3, 0.9,
    -0.5, -0.5, -0.5,   0.3, 0.3, 0.9,
    -0.5, -0.5, -0.5,   0.3, 0.3, 0.9,
    -0.5, -0.5,  0.5,   0.3, 0.3, 0.9,
    -0.5,  0.5,  0.5,   0.3, 0.3, 0.9,

     0.5,  0.5,  0.5,   0.9, 0.9, 0.3,
     0.5,  0.5, -0.5,   0.9, 0.9, 0.3,
     0.5, -0.5, -0.5,   0.9, 0.9, 0.3,
     0.5, -0.5, -0.5,   0.9, 0.9, 0.3,
     0.5, -0.5,  0.5,   0.9, 0.9, 0.3,
     0.5,  0.5,  0.5,   0.9, 0.9, 0.3,

    -0.5, -0.5, -0.5,   0.3, 0.9, 0.9,
     0.5, -0.5, -0.5,   0.3, 0.9, 0.9,
     0.5, -0.5,  0.5,   0.3, 0.9, 0.9,
     0.5, -0.5,  0.5,   0.3, 0.9, 0.9,
    -0.5, -0.5,  0.5,   0.3, 0.9, 0.9,
    -0.5, -0.5, -0.5,   0.3, 0.9, 0.9,

    -0.5,  0.5, -0.5,   0.9, 0.3, 0.9,
     0.5,  0.5, -0.5,   0.9, 0.3, 0.9,
     0.5,  0.5,  0.5,   0.9, 0.3, 0.9,
     0.5,  0.5,  0.5,   0.9, 0.3, 0.9,
    -0.5,  0.5,  0.5,   0.9, 0.3, 0.9,
    -0.5,  0.5, -0.5,   0.9, 0.3, 0.9,
];

fn main() {
    let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to init GLFW");
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

    let (mut window, events) = glfw
        .create_window(1024, 768, "Rust GMod Gravity Gun Engine", glfw::WindowMode::Windowed)
        .expect("Failed to create window");

    window.make_current();
    window.set_key_polling(true);
    window.set_mouse_button_polling(true); // Включаем опрос кнопок мыши
    window.set_scroll_polling(true);       // Включаем колесико мыши
    window.set_cursor_pos_polling(true);
    window.set_framebuffer_size_polling(true);
    window.set_cursor_mode(CursorMode::Disabled);

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    unsafe {
        gl::Enable(gl::DEPTH_TEST);
    }

    let shader = Shader::new(VERTEX_SHADER_SRC, FRAGMENT_SHADER_SRC);
    let cube_mesh = Mesh::new(CUBE_VERTICES);

    let physics_world = PhysicsWorld::new();
    let mut bodies: Vec<RigidBody> = Vec::new();

    // 0. Игрок
    let player_idx = 0;
    bodies.push(RigidBody::new_dynamic(
        Vec3::new(0.0, 3.0, 5.0),
        Vec3::new(0.4, 0.9, 0.4),
        70.0,
    ));

    // 1. Статическая платформа
    bodies.push(RigidBody::new_static(
        Vec3::new(0.0, -0.2, 0.0),
        Vec3::new(10.0, 0.2, 10.0),
    ));

    // 2. Динамические кубы
    bodies.push(RigidBody::new_dynamic(
        Vec3::new(1.0, 4.0, 0.0),
        Vec3::new(0.5, 0.5, 0.5),
        10.0,
    ));

    bodies.push(RigidBody::new_dynamic(
        Vec3::new(-1.5, 6.0, 0.5),
        Vec3::new(0.6, 0.6, 0.6),
        15.0,
    ));

    let mut camera = Camera::new(Vec3::ZERO);
    let mut last_frame_time = 0.0f32;
    let mut first_mouse = true;
    let mut last_x = 512.0f32;
    let mut last_y = 384.0f32;

    // --- Состояние Грави-пушки ---
    let mut held_object: Option<usize> = None;
    let mut hold_distance = 3.5f32; // Текущая дистанция удержания

    while !window.should_close() {
        let current_frame = glfw.get_time() as f32;
        let delta_time = current_frame - last_frame_time;
        last_frame_time = current_frame;

        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                // Нажатие 'E' — Спавн нового физического куба
                WindowEvent::Key(Key::E, _, Action::Press, _) => {
                    let spawn_pos = camera.position + camera.front * 1.5;
                    let new_cube = RigidBody::new_dynamic(
                        spawn_pos,
                        Vec3::new(0.4, 0.4, 0.4),
                        8.0,
                    );
                    bodies.push(new_cube);
                }

                // --- ГРАВИ-ПУШКА: ЛКМ (Захват / Отпускание) ---
                WindowEvent::MouseButton(MouseButton::Button1, Action::Press, _) => {
                    if held_object.is_none() {
                        if let Some((target_idx, dist)) = raycast_grab_target(&camera, &bodies, player_idx, 12.0) {
                            held_object = Some(target_idx);
                            hold_distance = dist.clamp(1.5, 8.0); // Запоминаем дистанцию захвата
                        }
                    }
                }
                WindowEvent::MouseButton(MouseButton::Button1, Action::Release, _) => {
                    if let Some(idx) = held_object {
                        bodies[idx].is_kinematic = false;
                        held_object = None;
                    }
                }

                // --- ГРАВИ-ПУШКА: ПКМ (Выстрел / Силовая волна) ---
                WindowEvent::MouseButton(MouseButton::Button2, Action::Press, _) => {
                    if let Some(idx) = held_object {
                        // Мощный выстрел удерживаемым кубом!
                        bodies[idx].is_kinematic = false;
                        bodies[idx].velocity = camera.front * 32.0; // Высокая скорость вылета
                        held_object = None;
                    } else {
                        // Гравитационная импульсная волна (растолкать все перед собой)
                        for (idx, body) in bodies.iter_mut().enumerate() {
                            if idx == player_idx || body.is_static() {
                                continue;
                            }
                            let to_body = body.position - camera.position;
                            if to_body.length() < 6.0 && camera.front.dot(to_body.normalize()) > 0.5 {
                                body.velocity += camera.front * 18.0 + Vec3::Y * 4.0;
                            }
                        }
                    }
                }

                // --- ГРАВИ-ПУШКА: Колесико мыши (Приближение/Отдаление куба) ---
                WindowEvent::Scroll(_x_offset, y_offset) => {
                    if held_object.is_some() {
                        hold_distance = (hold_distance + y_offset as f32 * 0.5).clamp(1.2, 10.0);
                    }
                }

                WindowEvent::CursorPos(xpos, ypos) => {
                    let (xpos, ypos) = (xpos as f32, ypos as f32);
                    if first_mouse {
                        last_x = xpos;
                        last_y = ypos;
                        first_mouse = false;
                    }

                    let x_offset = xpos - last_x;
                    let y_offset = last_y - ypos;
                    last_x = xpos;
                    last_y = ypos;

                    camera.process_mouse(x_offset, y_offset);
                }
                WindowEvent::FramebufferSize(w, h) => unsafe {
                    gl::Viewport(0, 0, w, h);
                },
                _ => {}
            }
        }

        // --- Управление игроком (Без скольжения) ---
        process_player_input(&window, &mut bodies[player_idx], &camera);

        // --- Обработка удержания предмета грави-пушкой ---
        if let Some(held_idx) = held_object {
            let target_pos = camera.position + camera.front * hold_distance;
            let body = &mut bodies[held_idx];
            body.is_kinematic = true;

            // Динамический физический притяг (куб сносит препятствия на пути)
            let delta = target_pos - body.position;
            body.velocity = delta * 24.0;
            body.position += body.velocity * delta_time;
        }

        // --- Физический шаг ---
        physics_world.step(&mut bodies, delta_time);

        // Камера в глазах игрока
        camera.position = bodies[player_idx].position + Vec3::new(0.0, 0.6, 0.0);

        unsafe {
            gl::ClearColor(0.1, 0.12, 0.15, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        shader.use_program();

        let (w, h) = window.get_size();
        let aspect = w as f32 / h.max(1) as f32;
        let projection = camera.get_projection_matrix(aspect);
        let view = camera.get_view_matrix();

        // Рендеринг физических объектов
        for (idx, body) in bodies.iter().enumerate() {
            if idx == player_idx {
                continue;
            }

            let model = Mat4::from_translation(body.position) * Mat4::from_scale(body.half_extents * 2.0);
            shader.set_mat4("uMVP", &(projection * view * model));
            cube_mesh.draw();
        }

        // Декоративный нефизический вращающийся куб
        let rot_cube_model = Mat4::from_translation(Vec3::new(0.0, 3.5, 0.0))
            * Mat4::from_rotation_y(current_frame * 1.5)
            * Mat4::from_rotation_x(current_frame * 0.8)
            * Mat4::from_scale(Vec3::splat(0.8));
        shader.set_mat4("uMVP", &(projection * view * rot_cube_model));
        cube_mesh.draw();

        window.swap_buffers();
    }
}

// Raycast алгоритм выстрела луча для грави-пушки
fn raycast_grab_target(
    camera: &Camera,
    bodies: &[RigidBody],
    player_idx: usize,
    max_dist: f32,
) -> Option<(usize, f32)> {
    let mut closest = None;
    let mut min_proj = max_dist;

    for (idx, body) in bodies.iter().enumerate() {
        if idx == player_idx || body.is_static() {
            continue;
        }

        let to_body = body.position - camera.position;
        let proj_len = to_body.dot(camera.front);

        if proj_len > 0.5 && proj_len < min_proj {
            // Перпендикулярное расстояние от луча взгляда до центра куба
            let perp_dist = (to_body - camera.front * proj_len).length();
            let body_radius = body.half_extents.max_element() + 0.3;

            if perp_dist <= body_radius {
                min_proj = proj_len;
                closest = Some((idx, proj_len));
            }
        }
    }

    closest
}

fn process_player_input(window: &glfw::Window, player: &mut RigidBody, camera: &Camera) {
    let speed = 7.5f32;
    let (front_xz, right_xz) = camera.get_move_vectors();
    let mut move_dir = Vec3::ZERO;

    if window.get_key(Key::W) == Action::Press {
        move_dir += front_xz;
    }
    if window.get_key(Key::S) == Action::Press {
        move_dir -= front_xz;
    }
    if window.get_key(Key::A) == Action::Press {
        move_dir -= right_xz;
    }
    if window.get_key(Key::D) == Action::Press {
        move_dir += right_xz;
    }

    if move_dir.length_squared() > 0.0 {
        move_dir = move_dir.normalize();
        player.velocity.x = move_dir.x * speed;
        player.velocity.z = move_dir.z * speed;
    } else {
        player.velocity.x = 0.0;
        player.velocity.z = 0.0;
    }

    if window.get_key(Key::Space) == Action::Press && player.is_grounded {
        player.velocity.y = 8.5;
        player.is_grounded = false;
    }
}