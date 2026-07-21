use glam::{Mat4, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub front: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub world_up: Vec3,

    pub yaw: f32,
    pub pitch: f32,
    pub sensitivity: f32,
    pub fov: f32,
}

impl Camera {
    pub fn new(position: Vec3) -> Self {
        let mut camera = Self {
            position,
            front: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::Y,
            right: Vec3::X,
            world_up: Vec3::Y,
            yaw: -90.0,
            pitch: 0.0,
            sensitivity: 0.1,
            fov: 45.0,
        };
        camera.update_vectors();
        camera
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.front, self.up)
    }

    pub fn get_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), aspect_ratio, 0.1, 100.0)
    }

    pub fn process_mouse(&mut self, mut x_offset: f32, mut y_offset: f32) {
        x_offset *= self.sensitivity;
        y_offset *= self.sensitivity;

        self.yaw += x_offset;
        self.pitch += y_offset;
        self.pitch = self.pitch.clamp(-89.0, 89.0);

        self.update_vectors();
    }

    // Возвращает вектор движения по горизонтали (XZ плоская проекция)
    pub fn get_move_vectors(&self) -> (Vec3, Vec3) {
        let front_xz = Vec3::new(self.front.x, 0.0, self.front.z).normalize_or_zero();
        let right_xz = Vec3::new(self.right.x, 0.0, self.right.z).normalize_or_zero();
        (front_xz, right_xz)
    }

    fn update_vectors(&mut self) {
        let front = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos(),
        );
        self.front = front.normalize();
        self.right = self.front.cross(self.world_up).normalize();
        self.up = self.right.cross(self.front).normalize();
    }
}