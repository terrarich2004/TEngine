use glam::Vec3;

#[derive(Clone, Debug)]
pub struct RigidBody {
    pub position: Vec3,
    pub velocity: Vec3,
    pub half_extents: Vec3, // Половина размера AABB куба
    pub mass: f32,         // 0.0 = Статический объект
    pub restitution: f32,  // Коэффициент упругости / отскока
    pub friction: f32,     // Трение поверхностей
    pub is_grounded: bool,
    pub is_kinematic: bool, // Если true — объект управляется вручную (например, когда его несут)
}

impl RigidBody {
    pub fn new_static(position: Vec3, half_extents: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            half_extents,
            mass: 0.0,
            restitution: 0.05,
            friction: 0.8,
            is_grounded: false,
            is_kinematic: false,
        }
    }

    pub fn new_dynamic(position: Vec3, half_extents: Vec3, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            half_extents,
            mass,
            restitution: 0.25,
            friction: 0.6,
            is_grounded: false,
            is_kinematic: false,
        }
    }

    pub fn is_static(&self) -> bool {
        self.mass <= 0.0 || self.is_kinematic
    }
}

pub struct PhysicsWorld {
    pub gravity: Vec3,
    pub sub_steps: usize, // Количество подшагов для идеальной точности
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            gravity: Vec3::new(0.0, -22.0, 0.0),
            sub_steps: 4, // 4 подшага за кадр для исключения тоннелирования
        }
    }

    pub fn step(&self, bodies: &mut [RigidBody], delta_time: f32) {
        // Делим кадр на микро-шаги для максимальной стабильности
        let dt = (delta_time.min(0.05)) / (self.sub_steps as f32);

        for _ in 0..self.sub_steps {
            // 1. Интеграция позиций и гравитации
            for body in bodies.iter_mut() {
                if body.is_static() || body.is_kinematic {
                    continue;
                }

                body.velocity += self.gravity * dt;

                // Линейное затухание скорости (сопротивление воздуха)
                body.velocity *= (1.0 - 0.15 * dt).max(0.0);

                body.position += body.velocity * dt;
                body.is_grounded = false;
            }

            // 2. Итеративное разрешение коллизий AABB vs AABB (MTV)
            let body_count = bodies.len();
            for i in 0..body_count {
                for j in (i + 1)..body_count {
                    let (first, second) = bodies.split_at_mut(j);
                    let a = &mut first[i];
                    let b = &mut second[0];

                    if (a.is_static() || a.is_kinematic) && (b.is_static() || b.is_kinematic) {
                        continue;
                    }

                    Self::resolve_collision(a, b);
                }
            }

            // 3. Гашение дрожания при покое на земле
            for body in bodies.iter_mut() {
                if body.is_grounded && body.velocity.y.abs() < 0.2 {
                    body.velocity.y = 0.0;
                }
            }
        }
    }

    fn resolve_collision(a: &mut RigidBody, b: &mut RigidBody) {
        let delta = a.position - b.position;
        let overlap_x = (a.half_extents.x + b.half_extents.x) - delta.x.abs();
        let overlap_y = (a.half_extents.y + b.half_extents.y) - delta.y.abs();
        let overlap_z = (a.half_extents.z + b.half_extents.z) - delta.z.abs();

        if overlap_x > 0.0 && overlap_y > 0.0 && overlap_z > 0.0 {
            let mut mtv = Vec3::ZERO;

            if overlap_x < overlap_y && overlap_x < overlap_z {
                mtv.x = if delta.x > 0.0 { overlap_x } else { -overlap_x };
            } else if overlap_y < overlap_z {
                mtv.y = if delta.y > 0.0 { overlap_y } else { -overlap_y };
            } else {
                mtv.z = if delta.z > 0.0 { overlap_z } else { -overlap_z };
            }

            let a_static = a.is_static() || a.is_kinematic;
            let b_static = b.is_static() || b.is_kinematic;

            if a_static {
                b.position -= mtv;
                Self::apply_response(b, -mtv.normalize(), a.restitution, a.friction);
            } else if b_static {
                a.position += mtv;
                Self::apply_response(a, mtv.normalize(), b.restitution, b.friction);
            } else {
                let total_mass = a.mass + b.mass;
                let a_ratio = b.mass / total_mass;
                let b_ratio = a.mass / total_mass;

                a.position += mtv * a_ratio;
                b.position -= mtv * b_ratio;

                let normal = mtv.normalize();
                Self::apply_response(a, normal, b.restitution, b.friction);
                Self::apply_response(b, -normal, a.restitution, a.friction);
            }

            if mtv.y > 0.0 {
                a.is_grounded = true;
            }
            if mtv.y < 0.0 {
                b.is_grounded = true;
            }
        }
    }

    fn apply_response(body: &mut RigidBody, normal: Vec3, other_restitution: f32, other_friction: f32) {
        let vel_along_normal = body.velocity.dot(normal);
        if vel_along_normal < 0.0 {
            let e = body.restitution.max(other_restitution);
            let impulse = -(1.0 + e) * vel_along_normal;
            body.velocity += normal * impulse;

            // Трение о поверхность
            let tangent = body.velocity - normal * body.velocity.dot(normal);
            if tangent.length_squared() > 0.0001 {
                let mu = body.friction.max(other_friction);
                body.velocity -= tangent * mu * 0.15;
            }
        }
    }
}