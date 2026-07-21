mod camera;
mod mesh;
mod physics;
mod shader;

use camera::Camera;
use glfw::{Action, Context, Key, MouseButton};
use glam::{Mat4, Vec3};
use mesh::Mesh;
use physics::{PhysicsWorld, RigidBody, AABB};
use shader::Shader;
use std::time::Instant;

const WALK_SPEED: f32 = 5.0;
const SPRINT_SPEED: f32 = 9.0;
const CROUCH_SPEED: f32 = 2.2;

const STAND_HALF_HEIGHT: f32 = 0.9;
const CROUCH_HALF_HEIGHT: f32 = 0.45;

const STAND_EYE_OFFSET: f32 = 0.7;
const CROUCH_EYE_OFFSET: f32 = 0.3;

fn main() {
    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();

    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));

    let (mut window, events) = glfw
        .create_window(1280, 720, "TEngine - Physics Engine", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.make_current();
    window.set_key_polling(true);
    window.set_mouse_button_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_scroll_polling(true);
    window.set_cursor_mode(glfw::CursorMode::Disabled);

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::ClearColor(0.1, 0.1, 0.15, 1.0);
    }

    let shader = Shader::new("shaders/vertex.glsl", "shaders/fragment.glsl");
    let cube_mesh = Mesh::create_cube();

    let mut physics_world = PhysicsWorld::new();

    // 1. Пол
    let ground = RigidBody::new(Vec3::new(0.0, -1.0, 0.0), Vec3::new(20.0, 1.0, 20.0), 0.0, true);
    physics_world.add_body(ground);

    // 2. Игрок
    let player_body = RigidBody::new(
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(0.4, STAND_HALF_HEIGHT, 0.4),
        70.0,
        false,
    );
    let player_id = physics_world.add_body(player_body);

    let mut camera = Camera::new(Vec3::new(0.0, 2.7, 0.0));

    let mut is_crouched = false;
    let mut current_eye_offset = STAND_EYE_OFFSET;

    let mut held_body_index: Option<usize> = None;
    let mut gravgun_distance: f32 = 3.5;

    let mut last_frame = Instant::now();
    let mut first_mouse = true;
    let mut last_x = 640.0;
    let mut last_y = 360.0;

    let mut e_pressed_last = false;
    let mut space_pressed_last = false;

    while !window.should_close() {
        let now = Instant::now();
        let delta_time = (now - last_frame).as_secs_f32().min(0.05);
        last_frame = now;

        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                glfw::WindowEvent::CursorPos(xpos, ypos) => {
                    if first_mouse {
                        last_x = xpos as f32;
                        last_y = ypos as f32;
                        first_mouse = false;
                    }

                    let xoffset = xpos as f32 - last_x;
                    let yoffset = last_y - ypos as f32;

                    last_x = xpos as f32;
                    last_y = ypos as f32;

                    camera.process_mouse_movement(xoffset, yoffset, true);
                }
                glfw::WindowEvent::Scroll(_, yoffset) => {
                    if held_body_index.is_some() {
                        gravgun_distance = (gravgun_distance + yoffset as f32 * 0.5).clamp(1.5, 10.0);
                    }
                }
                _ => {}
            }
        }

        // --- УПРАВЛЕНИЕ: БЕГ И ПРИСЕДАНИЕ ---
        let ctrl_pressed = window.get_key(Key::LeftControl) == Action::Press;
        let shift_pressed = window.get_key(Key::LeftShift) == Action::Press;

        let mut target_crouch = ctrl_pressed;

        // Head Check
        if is_crouched && !target_crouch {
            let player_b = &physics_world.bodies[player_id];
            let delta_y = STAND_HALF_HEIGHT - player_b.half_extents.y;

            let test_pos = Vec3::new(
                player_b.position.x,
                player_b.position.y + delta_y,
                player_b.position.z,
            );
            let test_extents = Vec3::new(
                player_b.half_extents.x,
                STAND_HALF_HEIGHT,
                player_b.half_extents.z,
            );
            let test_aabb = AABB::new(test_pos, test_extents);

            if physics_world.check_aabb_collision(&test_aabb, Some(player_id)) {
                target_crouch = true;
            }
        }

        if target_crouch != is_crouched {
            is_crouched = target_crouch;
            let target_half_height = if is_crouched {
                CROUCH_HALF_HEIGHT
            } else {
                STAND_HALF_HEIGHT
            };
            physics_world.bodies[player_id].set_height(target_half_height);
        }

        let current_speed = if is_crouched {
            CROUCH_SPEED
        } else if shift_pressed {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };

        // WASD
        let mut move_dir = Vec3::ZERO;
        let flat_front = Vec3::new(camera.front.x, 0.0, camera.front.z).normalize_or_zero();
        let flat_right = Vec3::new(camera.right.x, 0.0, camera.right.z).normalize_or_zero();

        if window.get_key(Key::W) == Action::Press { move_dir += flat_front; }
        if window.get_key(Key::S) == Action::Press { move_dir -= flat_front; }
        if window.get_key(Key::D) == Action::Press { move_dir += flat_right; }
        if window.get_key(Key::A) == Action::Press { move_dir -= flat_right; }

        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
        }

        let player = &mut physics_world.bodies[player_id];
        player.velocity.x = move_dir.x * current_speed;
        player.velocity.z = move_dir.z * current_speed;

        // Прыжок
        let space_pressed = window.get_key(Key::Space) == Action::Press;
        if space_pressed && !space_pressed_last && player.is_grounded {
            player.velocity.y = 5.5;
        }
        space_pressed_last = space_pressed;

        // Спавн куба
        let e_pressed = window.get_key(Key::E) == Action::Press;
        if e_pressed && !e_pressed_last {
            let spawn_pos = camera.position + camera.front * 2.5;
            let mut dynamic_cube = RigidBody::new(spawn_pos, Vec3::splat(0.4), 10.0, false);
            dynamic_cube.velocity = camera.front * 8.0;
            physics_world.add_body(dynamic_cube);
        }
        e_pressed_last = e_pressed;

        // Гравипушка
        let lmb_pressed = window.get_mouse_button(MouseButton::Button1) == Action::Press;
        let rmb_pressed = window.get_mouse_button(MouseButton::Button2) == Action::Press;

        if lmb_pressed {
            if held_body_index.is_none() {
                if let Some(hit) = physics_world.raycast(camera.position, camera.front, 8.0) {
                    if hit.body_index != player_id && hit.body_index != 0 {
                        held_body_index = Some(hit.body_index);
                        gravgun_distance = hit.distance.clamp(2.0, 8.0);
                    }
                }
            }
        } else {
            held_body_index = None;
        }

        if let Some(idx) = held_body_index {
            let target_pos = camera.position + camera.front * gravgun_distance;
            let body = &mut physics_world.bodies[idx];
            let delta = target_pos - body.position;
            body.velocity = delta * 12.0;

            if rmb_pressed {
                body.velocity = camera.front * 25.0;
                held_body_index = None;
            }
        } else if rmb_pressed {
            if let Some(hit) = physics_world.raycast(camera.position, camera.front, 5.0) {
                if hit.body_index != player_id && hit.body_index != 0 {
                    physics_world.bodies[hit.body_index].apply_impulse(camera.front * 200.0);
                }
            }
        }

        physics_world.step(delta_time);

        // Плавное приседание камеры
        let target_eye_offset = if is_crouched { CROUCH_EYE_OFFSET } else { STAND_EYE_OFFSET };
        current_eye_offset += (target_eye_offset - current_eye_offset) * (delta_time * 14.0).min(1.0);

        let player_pos = physics_world.bodies[player_id].position;
        camera.position = Vec3::new(player_pos.x, player_pos.y + current_eye_offset, player_pos.z);

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        shader.use_program();

        let projection = Mat4::perspective_rh_gl(
            camera.zoom.to_radians(),
            1280.0 / 720.0,
            0.1,
            100.0,
        );
        let view = camera.get_view_matrix();

        shader.set_mat4("projection", &projection);
        shader.set_mat4("view", &view);

        for (i, body) in physics_world.bodies.iter().enumerate() {
            if i == player_id {
                continue;
            }

            let model = Mat4::from_translation(body.position) * Mat4::from_scale(body.half_extents);
            shader.set_mat4("model", &model);

            if i == 0 {
                shader.set_vec3("objectColor", Vec3::new(0.3, 0.3, 0.35));
            } else if Some(i) == held_body_index {
                shader.set_vec3("objectColor", Vec3::new(0.2, 0.9, 0.3));
            } else {
                shader.set_vec3("objectColor", Vec3::new(0.8, 0.4, 0.2));
            }

            cube_mesh.draw();
        }

        let time = glfw.get_time() as f32;
        let float_pos = Vec3::new(0.0, 4.0 + (time * 2.0).sin() * 0.3, -5.0);
        let float_model = Mat4::from_translation(float_pos)
            * Mat4::from_rotation_y(time)
            * Mat4::from_rotation_x(time * 0.5)
            * Mat4::from_scale(Vec3::splat(0.5));

        shader.set_mat4("model", &float_model);
        shader.set_vec3("objectColor", Vec3::new(0.9, 0.2, 0.6));
        cube_mesh.draw();

        window.swap_buffers();
    }
}