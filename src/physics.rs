use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(position: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: position - half_extents,
            max: position + half_extents,
        }
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn get_mtv(&self, other: &AABB) -> Option<Vec3> {
        if !self.intersects(other) {
            return None;
        }

        let overlap_x1 = other.max.x - self.min.x;
        let overlap_x2 = self.max.x - other.min.x;
        let overlap_x = if overlap_x1 < overlap_x2 {
            overlap_x1
        } else {
            -overlap_x2
        };

        let overlap_y1 = other.max.y - self.min.y;
        let overlap_y2 = self.max.y - other.min.y;
        let overlap_y = if overlap_y1 < overlap_y2 {
            overlap_y1
        } else {
            -overlap_y2
        };

        let overlap_z1 = other.max.z - self.min.z;
        let overlap_z2 = self.max.z - other.min.z;
        let overlap_z = if overlap_z1 < overlap_z2 {
            overlap_z1
        } else {
            -overlap_z2
        };

        let abs_x = overlap_x.abs();
        let abs_y = overlap_y.abs();
        let abs_z = overlap_z.abs();

        if abs_x < abs_y && abs_x < abs_z {
            Some(Vec3::new(overlap_x, 0.0, 0.0))
        } else if abs_y < abs_z {
            Some(Vec3::new(0.0, overlap_y, 0.0))
        } else {
            Some(Vec3::new(0.0, 0.0, overlap_z))
        }
    }
}

#[allow(dead_code)]
pub struct RaycastHit {
    pub body_index: usize,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RigidBody {
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
    pub inv_mass: f32,
    pub half_extents: Vec3,
    pub aabb: AABB,
    pub is_kinematic: bool,
    pub is_grounded: bool,
    pub friction: f32,
    pub restitution: f32,
}

impl RigidBody {
    pub fn new(position: Vec3, half_extents: Vec3, mass: f32, is_kinematic: bool) -> Self {
        let inv_mass = if is_kinematic || mass <= 0.0 {
            0.0
        } else {
            1.0 / mass
        };
        let aabb = AABB::new(position, half_extents);

        Self {
            position,
            velocity: Vec3::ZERO,
            mass,
            inv_mass,
            half_extents,
            aabb,
            is_kinematic,
            is_grounded: false,
            friction: 0.5,    // Коэффициент трения
            restitution: 0.2, // Упругость отскока
        }
    }

    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if !self.is_kinematic {
            self.velocity += impulse * self.inv_mass;
        }
    }

    pub fn set_height(&mut self, new_half_height: f32) {
        let old_half_height = self.half_extents.y;
        if (old_half_height - new_half_height).abs() < 1e-4f32 {
            return;
        }

        let delta_y = new_half_height - old_half_height;
        self.position.y += delta_y;
        self.half_extents.y = new_half_height;
        self.aabb = AABB::new(self.position, self.half_extents);
    }
}

pub struct PhysicsWorld {
    pub bodies: Vec<RigidBody>,
    pub gravity: Vec3,
    pub sub_steps: usize,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            gravity: Vec3::new(0.0, -9.81, 0.0),
            sub_steps: 4,
        }
    }

    pub fn add_body(&mut self, body: RigidBody) -> usize {
        self.bodies.push(body);
        self.bodies.len() - 1
    }

    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let sub_dt = dt / self.sub_steps as f32;

        for _ in 0..self.sub_steps {
            self.sub_step(sub_dt);
        }
    }

    fn sub_step(&mut self, dt: f32) {
        let n = self.bodies.len();

        for body in self.bodies.iter_mut() {
            if !body.is_kinematic {
                // Применяем гравитацию
                body.velocity += self.gravity * dt;

                // 1. Линейное затухание (сопротивление среды/воздуха)
                let damping = (1.0 - 1.5 * dt).max(0.0);
                body.velocity.x *= damping;
                body.velocity.z *= damping;

                // 2. Порог покоя: если объект на полу и движется очень медленно — останавливаем полностью
                if body.is_grounded && body.velocity.length_squared() < 0.005 {
                    body.velocity.x = 0.0;
                    body.velocity.z = 0.0;
                }

                body.position += body.velocity * dt;
                body.is_grounded = false;
            }
            body.aabb = AABB::new(body.position, body.half_extents);
        }

        let iterations = 4;
        for _ in 0..iterations {
            for i in 0..n {
                for j in (i + 1)..n {
                    self.resolve_collision(i, j);
                }
            }
        }
    }

    fn resolve_collision(&mut self, i: usize, j: usize) {
        if i == j {
            return;
        }

        let (body_a, body_b) = if i < j {
            let (left, right) = self.bodies.split_at_mut(j);
            (&mut left[i], &mut right[0])
        } else {
            let (left, right) = self.bodies.split_at_mut(i);
            (&mut right[0], &mut left[j])
        };

        if body_a.is_kinematic && body_b.is_kinematic {
            return;
        }

        if let Some(mtv) = body_a.aabb.get_mtv(&body_b.aabb) {
            let total_inv_mass = body_a.inv_mass + body_b.inv_mass;
            if total_inv_mass <= 0.0 {
                return;
            }

            let normal = mtv.normalize_or_zero();

            // Коррекция позиции (разделение объектов)
            let correction = mtv;
            if !body_a.is_kinematic {
                body_a.position += correction * (body_a.inv_mass / total_inv_mass);
                body_a.aabb = AABB::new(body_a.position, body_a.half_extents);
            }
            if !body_b.is_kinematic {
                body_b.position -= correction * (body_b.inv_mass / total_inv_mass);
                body_b.aabb = AABB::new(body_b.position, body_b.half_extents);
            }

            // Фиксация касания пола
            if normal.y > 0.7 && !body_a.is_kinematic {
                body_a.is_grounded = true;
            }
            if normal.y < -0.7 && !body_b.is_kinematic {
                body_b.is_grounded = true;
            }

            // Относительная скорость
            let relative_velocity = body_a.velocity - body_b.velocity;
            let vel_along_normal = relative_velocity.dot(normal);

            if vel_along_normal < 0.0 {
                // 1. Нормальный импульс (Отскок)
                let e = body_a.restitution.min(body_b.restitution);
                let j_normal = -(1.0 + e) * vel_along_normal / total_inv_mass;
                let normal_impulse = normal * j_normal;

                if !body_a.is_kinematic {
                    body_a.velocity += normal_impulse * body_a.inv_mass;
                }
                if !body_b.is_kinematic {
                    body_b.velocity -= normal_impulse * body_b.inv_mass;
                }

                // 2. Тангенциальный импульс (Трение по закону Кулона)
                let updated_rel_vel = body_a.velocity - body_b.velocity;
                let tangent = updated_rel_vel - normal * updated_rel_vel.dot(normal);
                let tangent_len = tangent.length();

                if tangent_len > 1e-4 {
                    let tangent_dir = tangent / tangent_len;
                    let friction_coef = (body_a.friction * body_b.friction).sqrt();

                    // Ограничиваем трение максимальной силой Кулона: F_fric <= mu * N
                    let j_friction = (-tangent_len / total_inv_mass).max(-friction_coef * j_normal);
                    let friction_impulse = tangent_dir * j_friction;

                    if !body_a.is_kinematic {
                        body_a.velocity += friction_impulse * body_a.inv_mass;
                    }
                    if !body_b.is_kinematic {
                        body_b.velocity -= friction_impulse * body_b.inv_mass;
                    }
                }
            }
        }
    }

    pub fn check_aabb_collision(&self, test_aabb: &AABB, ignore_idx: Option<usize>) -> bool {
        for (i, body) in self.bodies.iter().enumerate() {
            if Some(i) == ignore_idx {
                continue;
            }
            if body.aabb.intersects(test_aabb) {
                return true;
            }
        }
        false
    }

    pub fn raycast(&self, ray_origin: Vec3, ray_dir: Vec3, max_distance: f32) -> Option<RaycastHit> {
        let mut closest_hit: Option<RaycastHit> = None;
        let mut min_dist = max_distance;

        for (i, body) in self.bodies.iter().enumerate() {
            if let Some((dist, normal)) = ray_aabb_intersection(ray_origin, ray_dir, &body.aabb) {
                if dist >= 0.0 && dist < min_dist {
                    min_dist = dist;
                    closest_hit = Some(RaycastHit {
                        body_index: i,
                        point: ray_origin + ray_dir * dist,
                        normal,
                        distance: dist,
                    });
                }
            }
        }

        closest_hit
    }
}

fn ray_aabb_intersection(origin: Vec3, dir: Vec3, aabb: &AABB) -> Option<(f32, Vec3)> {
    let eps = 1e-8f32;
    let mut tmin = (aabb.min.x - origin.x) / (if dir.x != 0.0 { dir.x } else { eps });
    let mut tmax = (aabb.max.x - origin.x) / (if dir.x != 0.0 { dir.x } else { eps });
    let mut normal_x = if dir.x > 0.0 { Vec3::NEG_X } else { Vec3::X };

    if tmin > tmax {
        std::mem::swap(&mut tmin, &mut tmax);
        normal_x = -normal_x;
    }

    let mut tymin = (aabb.min.y - origin.y) / (if dir.y != 0.0 { dir.y } else { eps });
    let mut tymax = (aabb.max.y - origin.y) / (if dir.y != 0.0 { dir.y } else { eps });
    let mut normal_y = if dir.y > 0.0 { Vec3::NEG_Y } else { Vec3::Y };

    if tymin > tymax {
        std::mem::swap(&mut tymin, &mut tymax);
        normal_y = -normal_y;
    }

    if (tmin > tymax) || (tymin > tmax) {
        return None;
    }

    let mut norm = normal_x;
    if tymin > tmin {
        tmin = tymin;
        norm = normal_y;
    }
    if tymax < tmax {
        tmax = tymax;
    }

    let mut tzmin = (aabb.min.z - origin.z) / (if dir.z != 0.0 { dir.z } else { eps });
    let mut tzmax = (aabb.max.z - origin.z) / (if dir.z != 0.0 { dir.z } else { eps });
    let mut normal_z = if dir.z > 0.0 { Vec3::NEG_Z } else { Vec3::Z };

    if tzmin > tzmax {
        std::mem::swap(&mut tzmin, &mut tzmax);
        normal_z = -normal_z;
    }

    if (tmin > tzmax) || (tzmin > tmax) {
        return None;
    }

    if tzmin > tmin {
        tmin = tzmin;
        norm = normal_z;
    }
    if tzmax < tmax {
        tmax = tzmax;
    }

    if tmax < 0.0 {
        return None;
    }

    Some((tmin, norm))
}