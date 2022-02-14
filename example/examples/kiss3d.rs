use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use anyhow::{bail, Result};
use clap::Clap;
use kiss3d::{light::Light, nalgebra as na, scene::SceneNode, window::Window};
use mesh_loader::stl::{FromStl, Triangle};
use na::{Translation3, UnitQuaternion, Vector3};
use tracing::debug;

#[derive(Debug, Clap)]
struct Args {
    #[clap(parse(from_os_str))]
    path: PathBuf,
    #[clap(long, value_name = "X,Y,Z", default_value = "0.1,0.1,0.1")]
    scale: Scale,
}

#[derive(Debug)]
struct Scale(f32, f32, f32);

impl FromStr for Scale {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.trim().splitn(3, ',');
        Ok(Scale(
            iter.next().unwrap().parse()?,
            iter.next().unwrap().parse()?,
            iter.next().unwrap().parse()?,
        ))
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    debug!(?args);
    let path = &args.path;
    let scale = Vector3::new(args.scale.0, args.scale.1, args.scale.2);

    let mut window = Window::new(&format!("{} ー Meshes Example", args.path.display()));

    let mut base = match path.extension().and_then(OsStr::to_str) {
        Some("stl" | "STL") => add_stl(&mut window, path, scale)?,
        Some("dae" | "DAE") => add_collada(&mut window, path, scale)?,
        _ => bail!("unsupported file type {path:?}"),
    };
    base.set_local_scale(args.scale.0, args.scale.1, args.scale.2);

    base.append_translation(&Translation3::new(0.0, -0.05, -0.2));

    window.set_light(Light::StickToCamera);
    // window.set_background_color(0.5, 0.5, 0.5);

    let rot_triangle = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.014);

    let eye = na::Point3::new(3.0f32, 1.0, 1.0);
    let at = na::Point3::new(0.0f32, 0.0, 0.0);
    let mut camera = kiss3d::camera::ArcBall::new(eye, at);
    camera.set_up_axis(na::Vector3::z());
    camera.set_dist_step(0.5);
    while window.render_with_camera(&mut camera) {
        base.prepend_to_local_rotation(&rot_triangle);
    }

    Ok(())
}

fn add_collada(
    window: &mut Window,
    path: impl AsRef<Path>,
    scale: na::Vector3<f32>,
) -> Result<SceneNode> {
    let path = path.as_ref();
    let mut base = window.add_group();
    let collada = mesh_loader::collada::from_str(&fs::read_to_string(path)?)?;
    for mesh in mesh_loader::collada::instance::build_meshes(&collada) {
        debug!(
            "name={},vertices={},normals={},texcoords0={},texcoords1={},faces={}",
            mesh.name,
            mesh.vertices.len(),
            mesh.normals.len(),
            mesh.texcoords[0].len(),
            mesh.texcoords[1].len(),
            mesh.faces.len()
        );
        let positions = mesh.vertices.iter().map(|&v| na::Point3::from(v)).collect();
        let normals = if mesh.normals.is_empty() {
            None
        } else {
            Some(mesh.normals.iter().map(|&v| na::Vector3::from(v)).collect())
        };
        let texcoords = if mesh.texcoords[0].is_empty() {
            None
        } else {
            Some(
                mesh.texcoords[0]
                    .iter()
                    .map(|&v| na::Point2::from(v))
                    .collect(),
            )
        };
        let faces = mesh
            .faces
            .iter()
            .map(|v| na::Point3::new(v[0] as u16, v[1] as u16, v[2] as u16))
            .collect();
        let mut _scene = base.add_mesh(
            Rc::new(RefCell::new(kiss3d::resource::Mesh::new(
                positions, faces, normals, texcoords, false,
            ))),
            scale,
        );

        // if let Some(path) = materials.get(0) {
        //     scene.set_texture_from_file(path, path.to_str().unwrap());
        // }
    }
    Ok(base)
}

fn add_stl(
    window: &mut Window,
    path: impl AsRef<Path>,
    scale: na::Vector3<f32>,
) -> io::Result<SceneNode> {
    let stl: Kiss3dMesh = mesh_loader::stl::from_slice(&fs::read(path)?)?;
    let mesh = Rc::new(RefCell::new(kiss3d::resource::Mesh::from(stl)));
    Ok(window.add_mesh(mesh, scale))
}

#[derive(Default)]
struct Kiss3dMesh {
    coords: Vec<na::Point3<f32>>,
    faces: Vec<na::Point3<u16>>,
    normals: Vec<na::Vector3<f32>>,
}

// Not public API.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
#[derive(Default)]
struct MeshReadContext {
    mesh: Kiss3dMesh,
    vertices_to_indices: HashMap<[u32; 3], usize>,
    vertices_indices: [usize; 3],
}

impl FromStl for Kiss3dMesh {
    type Context = MeshReadContext;

    fn start() -> Self::Context {
        MeshReadContext::default()
    }

    fn end(mut cx: Self::Context) -> Self {
        cx.mesh.coords.shrink_to_fit();
        cx.mesh.faces.shrink_to_fit();
        cx.mesh.normals.shrink_to_fit();
        cx.mesh
    }

    fn push_triangle(cx: &mut Self::Context, triangle: Triangle) {
        for (i, vertex) in triangle.vertices.iter().enumerate() {
            let bits = [
                vertex[0].to_bits(),
                vertex[1].to_bits(),
                vertex[2].to_bits(),
            ];

            if let Some(&index) = cx.vertices_to_indices.get(&bits) {
                cx.vertices_indices[i] = index;
            } else {
                let index = cx.mesh.coords.len();
                cx.vertices_to_indices.insert(bits, index);
                cx.vertices_indices[i] = index;
                cx.mesh.coords.push((*vertex).into());
            }
        }

        cx.mesh.normals.push(triangle.normal.into());
        cx.mesh.faces.push(na::Point3::new(
            cx.vertices_indices[0] as u16,
            cx.vertices_indices[1] as u16,
            cx.vertices_indices[2] as u16,
        ));
    }

    fn reserve(cx: &mut Self::Context, num_triangles: u32) {
        // Use reserve_exact because binary stl has information on the exact number of triangles.
        cx.mesh.faces.reserve_exact(num_triangles as _);
        cx.mesh.normals.reserve_exact(num_triangles as _);
        // The number of vertices can be up to three times the number of triangles,
        // but is usually less than the number of triangles because of deduplication.
        let cap = (num_triangles as f64 / 1.6) as usize;
        cx.mesh.coords.reserve(cap);
        cx.vertices_to_indices.reserve(cap);
    }
}

impl From<Kiss3dMesh> for kiss3d::resource::Mesh {
    fn from(mesh: Kiss3dMesh) -> Self {
        Self::new(mesh.coords, mesh.faces, Some(mesh.normals), None, false)
    }
}
