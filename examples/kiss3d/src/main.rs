#[path = "../../../tests/shared/assimp.rs"]
mod assimp_helper;

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use kiss3d::{light::Light, nalgebra as na, scene::SceneNode, window::Window};
use lexopt::prelude::*;
use na::{Translation3, UnitQuaternion, Vector3};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

const DEFAULT_SCALE: f32 = 1.;

#[derive(Debug)]
struct Args {
    path: PathBuf,
    scale: f32,
    // debug flag to check the difference between assimp.
    assimp: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut parser = lexopt::Parser::from_env();
        let mut path = None;
        let mut scale = None;
        let mut assimp = false;
        while let Some(arg) = parser.next()? {
            match arg {
                Value(v) => path = Some(v.into()),
                Long("scale") => scale = Some(parser.value()?.parse()?),
                Long("assimp") => assimp = true,
                Short('h') | Long("help") => {
                    path = None;
                    break;
                }
                arg => return Err(arg.unexpected().into()),
            }
        }
        let Some(path) = path else {
            println!(
                "Usage: cargo run --bin {} -- <PATH> [--scale <SCALE={DEFAULT_SCALE}>]",
                env!("CARGO_BIN_NAME")
            );
            std::process::exit(1);
        };
        Ok(Self {
            path,
            scale: scale.unwrap_or(DEFAULT_SCALE),
            assimp,
        })
    }
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    eprintln!("args={args:?}");
    let path = &args.path;
    let scale = Vector3::new(args.scale, args.scale, args.scale);

    let mut window = Window::new(&format!("{} ー mesh-loader example", args.path.display()));

    let mut base = if args.assimp {
        add_assimp_mesh(&mut window, path, scale)
    } else {
        add_mesh(&mut window, path, scale)
    };
    base.set_local_scale(args.scale, args.scale, args.scale);

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

fn add_mesh(window: &mut Window, path: &Path, scale: na::Vector3<f32>) -> SceneNode {
    let loader = mesh_loader::Loader::default();
    let scene = loader.load(path).unwrap();
    scene_to_kiss3d_scene(window, scene, scale)
}

fn add_assimp_mesh(window: &mut Window, path: &Path, scale: na::Vector3<f32>) -> SceneNode {
    let mut importer = assimp::Importer::new();
    importer.pre_transform_vertices(|x| x.enable = true);
    importer.collada_ignore_up_direction(true);
    importer.triangulate(true);
    let ai_scene = importer.read_file(path.to_str().unwrap()).unwrap();
    let scene = assimp_helper::assimp_scene_to_scene(&ai_scene);
    scene_to_kiss3d_scene(window, scene, scale)
}

fn scene_to_kiss3d_scene(
    window: &mut Window,
    scene: mesh_loader::Scene,
    scale: na::Vector3<f32>,
) -> SceneNode {
    let mut base = window.add_group();
    assert_eq!(scene.meshes.len(), scene.materials.len());
    for (mesh, material) in scene.meshes.into_iter().zip(scene.materials) {
        eprintln!("mesh={mesh:?}");
        eprintln!("material={material:?}");
        let coords = mesh.vertices.into_iter().map(Into::into).collect();
        let faces = mesh
            .faces
            .into_iter()
            .map(|f| na::Point3::new(f[0], f[1], f[2]))
            .collect();
        let normals = if mesh.normals.is_empty() {
            None
        } else {
            Some(mesh.normals.into_iter().map(Into::into).collect())
        };
        let uvs = if mesh.texcoords[0].is_empty() {
            None
        } else {
            Some(mesh.texcoords[0].iter().copied().map(Into::into).collect())
        };
        let kiss3d_mesh = Rc::new(RefCell::new(kiss3d::resource::Mesh::new(
            coords, faces, normals, uvs, false,
        )));
        let mut kiss3d_scene = base.add_mesh(kiss3d_mesh, scale);
        if let Some(color) = material.color.diffuse {
            kiss3d_scene.set_color(color[0], color[1], color[2]);
        }
        if let Some(path) = &material.texture.diffuse {
            kiss3d_scene.set_texture_from_file(path, path.to_str().unwrap());
        }
        if let Some(path) = &material.texture.ambient {
            kiss3d_scene.set_texture_from_file(path, path.to_str().unwrap());
        }
    }
    base
}
