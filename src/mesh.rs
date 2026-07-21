// src/mesh.rs
use gl::types::*;
use glam::Vec3;
use std::fs;
use std::path::Path;

pub struct Mesh {
    pub vao: u32,
    pub vbo: u32,
    pub vertex_count: i32,
}

impl Mesh {
    /// Создает сетку из чередующегося массива данных:
    /// [x, y, z, nx, ny, nz, u, v]
    pub fn new_interleaved(vertices: &[f32], count: i32) -> Self {
        let mut vao = 0;
        let mut vbo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            let stride = (8 * std::mem::size_of::<f32>()) as GLsizei;

            // 0: Position
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());

            // 1: Normal
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * std::mem::size_of::<f32>()) as *const _,
            );

            // 2: TexCoord
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (6 * std::mem::size_of::<f32>()) as *const _,
            );

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        Self {
            vao,
            vbo,
            vertex_count: count,
        }
    }

    /// Загружает .obj файл, центрирует геометрию и автоматически рассчитывает габариты AABB коллизии.
    pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<(Self, Vec3), String> {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Ошибка чтения файла {:?}: {}", path.as_ref(), e))?;

        let mut positions: Vec<Vec3> = Vec::new();
        let mut normals: Vec<Vec3> = Vec::new();
        let mut tex_coords: Vec<[f32; 2]> = Vec::new();

        let mut min_p = Vec3::splat(f32::INFINITY);
        let mut max_p = Vec3::splat(f32::NEG_INFINITY);

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            let mut parts = line.split_whitespace();
            let prefix = match parts.next() {
                Some(p) => p,
                None => continue,
            };

            match prefix {
                "v" => {
                    let x: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let z: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let pos = Vec3::new(x, y, z);
                    min_p = min_p.min(pos);
                    max_p = max_p.max(pos);
                    positions.push(pos);
                }
                "vn" => {
                    let x: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let z: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    normals.push(Vec3::new(x, y, z));
                }
                "vt" => {
                    let u: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    let v: f32 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
                    tex_coords.push([u, v]);
                }
                _ => {}
            }
        }

        if positions.is_empty() {
            return Err("Файл OBJ не содержит вершин".into());
        }

        // Точный размер AABB коллайдера
        let size = max_p - min_p;
        // Центр AABB для центрирования сетки относительно физического тела
        let center = (min_p + max_p) * 0.5;

        let mut interleaved_data: Vec<f32> = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if !line.starts_with("f ") {
                continue;
            }

            let tokens: Vec<&str> = line.split_whitespace().skip(1).collect();
            if tokens.len() < 3 {
                continue;
            }

            let mut face_indices = Vec::new();
            for token in tokens {
                let parts: Vec<&str> = token.split('/').collect();
                let v_idx = parts.get(0).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1) - 1;
                let vt_idx = parts.get(1).and_then(|s| s.parse::<usize>().ok()).map(|i| i - 1);
                let vn_idx = parts.get(2).and_then(|s| s.parse::<usize>().ok()).map(|i| i - 1);
                face_indices.push((v_idx, vt_idx, vn_idx));
            }

            // Безопасная триангуляция для многоугольников (N-gons)
            for i in 1..face_indices.len() - 1 {
                let tri = [face_indices[0], face_indices[i], face_indices[i + 1]];
                for (v_idx, vt_idx, vn_idx) in tri {
                    // Центрируем вершину относительно центра AABB
                    let pos = positions.get(v_idx).copied().unwrap_or(Vec3::ZERO) - center;
                    let norm = vn_idx.and_then(|idx| normals.get(idx)).copied().unwrap_or(Vec3::Y);
                    let uv = vt_idx.and_then(|idx| tex_coords.get(idx)).copied().unwrap_or([0.0, 0.0]);

                    interleaved_data.extend_from_slice(&[
                        pos.x, pos.y, pos.z,
                        norm.x, norm.y, norm.z,
                        uv[0], uv[1],
                    ]);
                }
            }
        }

        let vertex_count = (interleaved_data.len() / 8) as i32;
        let mesh = Self::new_interleaved(&interleaved_data, vertex_count);

        Ok((mesh, size))
    }

    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, self.vertex_count);
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
        }
    }
}