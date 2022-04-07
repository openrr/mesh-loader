use std::{
    cell::RefCell,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use anyhow::{bail, Result};
use clap::Parser;
use kiss3d::{light::Light, nalgebra as na, scene::SceneNode, window::Window};
use na::{Translation3, UnitQuaternion, Vector3};
use tracing::debug;

#[derive(Debug, Parser)]
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
        // Some("obj" | "OBJ") => add_obj(&mut window, path, scale)?,
        _ => bail!("unsupported file type {path:?}"),
    };
    base.set_local_scale(args.scale.0, args.scale.1, args.scale.2);

    base.append_translation(&Translation3::new(0.0, -0.05, -0.2));

    window.set_light(Light::StickToCamera);

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

fn add_stl(
    window: &mut Window,
    path: impl AsRef<Path>,
    scale: na::Vector3<f32>,
) -> io::Result<SceneNode> {
    let stl = mesh_loader::stl::from_slice(&fs::read(path)?)?;
    let mesh = kiss3d::resource::Mesh::new(
        stl.vertices.into_iter().map(Into::into).collect(),
        stl.faces
            .into_iter()
            .map(|f| {
                na::Point3::new(
                    f[0].try_into().unwrap(),
                    f[1].try_into().unwrap(),
                    f[2].try_into().unwrap(),
                )
            })
            .collect(),
        Some(stl.normals.into_iter().map(Into::into).collect()),
        None,
        false,
    );
    let mesh = Rc::new(RefCell::new(mesh));
    Ok(window.add_mesh(mesh, scale))
}

fn add_collada(
    window: &mut Window,
    path: impl AsRef<Path>,
    scale: na::Vector3<f32>,
) -> Result<SceneNode> {
    let path = path.as_ref();
    let mut base = window.add_group();
    let collada = mesh_loader::collada::from_str(&fs::read_to_string(path)?)?;
    for mesh in collada.meshes {
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

        // TODO(material)
        // if let Some(path) = materials.get(0) {
        //     scene.set_texture_from_file(path, path.to_str().unwrap());
        // }
    }
    Ok(base)
}
