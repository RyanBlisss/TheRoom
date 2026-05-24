use gl::types::*;
use nalgebra_glm as glm;
use std::ffi::CString;
use tobj;

pub struct Shader {
    pub id: u32,
}

impl Shader {
    pub fn new(vert_src: &str, frag_src: &str) -> Self {
        unsafe {
            let vert = compile_shader(vert_src, gl::VERTEX_SHADER);
            let frag = compile_shader(frag_src, gl::FRAGMENT_SHADER);
            let id = gl::CreateProgram();
            gl::AttachShader(id, vert);
            gl::AttachShader(id, frag);
            gl::LinkProgram(id);
            gl::DeleteShader(vert);
            gl::DeleteShader(frag);
            Self { id }
        }
    }

    pub fn use_program(&self) {
        unsafe { gl::UseProgram(self.id); }
    }

    pub fn set_mat4(&self, name: &str, mat: &glm::Mat4) {
        unsafe {
            let cname = CString::new(name).unwrap();
            let loc = gl::GetUniformLocation(self.id, cname.as_ptr());
            gl::UniformMatrix4fv(loc, 1, gl::FALSE, mat.as_ptr());
        }
    }

    pub fn set_vec3(&self, name: &str, v: &glm::Vec3) {
        unsafe {
            let cname = CString::new(name).unwrap();
            let loc = gl::GetUniformLocation(self.id, cname.as_ptr());
            gl::Uniform3f(loc, v.x, v.y, v.z);
        }
    }

    pub fn set_float(&self, name: &str, v: f32) {
        unsafe {
            let cname = CString::new(name).unwrap();
            let loc = gl::GetUniformLocation(self.id, cname.as_ptr());
            gl::Uniform1f(loc, v);
        }
    }
}

unsafe fn compile_shader(src: &str, kind: GLenum) -> u32 {
    let shader = gl::CreateShader(kind);
    let csrc = CString::new(src).unwrap();
    gl::ShaderSource(shader, 1, &csrc.as_ptr(), std::ptr::null());
    gl::CompileShader(shader);

    let mut ok = gl::FALSE as i32;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut ok);
    if ok == gl::FALSE as i32 {
        let mut len = 0i32;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = vec![0u8; len as usize];
        gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buf.as_mut_ptr() as *mut i8);
        panic!("Shader compile error: {}", String::from_utf8_lossy(&buf));
    }
    shader
}

/// A single drawable mesh (VAO + VBO + EBO).
pub struct Mesh {
    pub vao: u32,
    vbo: u32,
    ebo: u32,
    pub index_count: i32,
    pub color: glm::Vec3,
}

impl Mesh {
    /// vertices: [x,y,z, nx,ny,nz, u,v, ...]
    pub fn new(vertices: &[f32], indices: &[u32], color: glm::Vec3) -> Self {
        let (mut vao, mut vbo, mut ebo) = (0u32, 0u32, 0u32);
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * 4) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * 4) as isize,
                indices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            let stride = (8 * std::mem::size_of::<f32>()) as i32;
            // position
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, 0 as *const _);
            gl::EnableVertexAttribArray(0);
            // normal
            gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE, stride, (3 * 4) as *const _);
            gl::EnableVertexAttribArray(1);
            // uv
            gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, stride, (6 * 4) as *const _);
            gl::EnableVertexAttribArray(2);

            gl::BindVertexArray(0);
        }
        Self { vao, vbo, ebo, index_count: indices.len() as i32, color }
    }

    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawElements(gl::TRIANGLES, self.index_count, gl::UNSIGNED_INT, std::ptr::null());
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

/// Build a box (room walls/floor/ceiling) from min/max corners.
/// Returns (vertices, indices).
pub fn build_box(min: glm::Vec3, max: glm::Vec3) -> (Vec<f32>, Vec<u32>) {
    let (x0, y0, z0) = (min.x, min.y, min.z);
    let (x1, y1, z1) = (max.x, max.y, max.z);

    // Each face: 4 verts * 8 floats (pos3 + norm3 + uv2)
    // Normals point INWARD so the interior faces are front-facing
    // (player is always inside the box looking at inside surfaces).
    #[rustfmt::skip]
    let verts: Vec<f32> = vec![
        // Floor (y = y0) — normal up +Y, seen from above
        x0,y0,z0, 0.0,1.0,0.0, 0.0,0.0,
        x0,y0,z1, 0.0,1.0,0.0, 0.0,1.0,
        x1,y0,z1, 0.0,1.0,0.0, 1.0,1.0,
        x1,y0,z0, 0.0,1.0,0.0, 1.0,0.0,
        // Ceiling (y = y1) — normal down -Y, seen from below
        x0,y1,z0, 0.0,-1.0,0.0, 0.0,0.0,
        x1,y1,z0, 0.0,-1.0,0.0, 1.0,0.0,
        x1,y1,z1, 0.0,-1.0,0.0, 1.0,1.0,
        x0,y1,z1, 0.0,-1.0,0.0, 0.0,1.0,
        // Front wall (z = z1) — normal inward -Z
        x0,y0,z1, 0.0,0.0,-1.0, 0.0,0.0,
        x0,y1,z1, 0.0,0.0,-1.0, 0.0,1.0,
        x1,y1,z1, 0.0,0.0,-1.0, 1.0,1.0,
        x1,y0,z1, 0.0,0.0,-1.0, 1.0,0.0,
        // Back wall (z = z0) — normal inward +Z
        x1,y0,z0, 0.0,0.0,1.0, 0.0,0.0,
        x1,y1,z0, 0.0,0.0,1.0, 0.0,1.0,
        x0,y1,z0, 0.0,0.0,1.0, 1.0,1.0,
        x0,y0,z0, 0.0,0.0,1.0, 1.0,0.0,
        // Left wall (x = x0) — normal inward +X
        x0,y0,z0, 1.0,0.0,0.0, 0.0,0.0,
        x0,y1,z0, 1.0,0.0,0.0, 0.0,1.0,
        x0,y1,z1, 1.0,0.0,0.0, 1.0,1.0,
        x0,y0,z1, 1.0,0.0,0.0, 1.0,0.0,
        // Right wall (x = x1) — normal inward -X
        x1,y0,z1, -1.0,0.0,0.0, 0.0,0.0,
        x1,y1,z1, -1.0,0.0,0.0, 0.0,1.0,
        x1,y1,z0, -1.0,0.0,0.0, 1.0,1.0,
        x1,y0,z0, -1.0,0.0,0.0, 1.0,0.0,
    ];

    let mut indices = Vec::new();
    for face in 0..6u32 {
        let b = face * 4;
        indices.extend_from_slice(&[b,b+1,b+2, b,b+2,b+3]);
    }
    (verts, indices)
}

/// Try to load the first mesh from an OBJ file.
/// Returns None if the file doesn't exist or can't be parsed.
/// Vertex format: pos(3) + normal(3) + uv(2), single-indexed.
pub fn load_obj_mesh(path: &str, color: glm::Vec3) -> Option<Mesh> {
    let (models, _) = tobj::load_obj(path, &tobj::LoadOptions {
        single_index: true,
        triangulate:  true,
        ..Default::default()
    }).ok()?;

    let model = models.into_iter().next()?;
    let m = &model.mesh;
    if m.positions.is_empty() { return None; }

    let vertex_count = m.positions.len() / 3;
    let mut verts = Vec::with_capacity(vertex_count * 8);
    for i in 0..vertex_count {
        let px = m.positions[i*3];
        let py = m.positions[i*3+1];
        let pz = m.positions[i*3+2];
        let (nx, ny, nz) = if m.normals.len() >= (i+1)*3 {
            (m.normals[i*3], m.normals[i*3+1], m.normals[i*3+2])
        } else { (0.0, 1.0, 0.0) };
        let (u, v) = if m.texcoords.len() >= (i+1)*2 {
            (m.texcoords[i*2], m.texcoords[i*2+1])
        } else { (0.0, 0.0) };
        verts.extend_from_slice(&[px, py, pz, nx, ny, nz, u, v]);
    }

    Some(Mesh::new(&verts, &m.indices, color))
}

/// Build a flat quad for a door opening (used as a coloured door plane).
pub fn build_quad(p0: glm::Vec3, p1: glm::Vec3, p2: glm::Vec3, p3: glm::Vec3, normal: glm::Vec3) -> (Vec<f32>, Vec<u32>) {
    #[rustfmt::skip]
    let verts = vec![
        p0.x,p0.y,p0.z, normal.x,normal.y,normal.z, 0.0,0.0,
        p1.x,p1.y,p1.z, normal.x,normal.y,normal.z, 1.0,0.0,
        p2.x,p2.y,p2.z, normal.x,normal.y,normal.z, 1.0,1.0,
        p3.x,p3.y,p3.z, normal.x,normal.y,normal.z, 0.0,1.0,
    ];
    let indices = vec![0,1,2, 0,2,3];
    (verts, indices)
}
