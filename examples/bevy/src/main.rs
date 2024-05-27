#![allow(clippy::needless_pass_by_value)]

use std::{f32::consts::PI, path::PathBuf};

use anyhow::Result;
use bevy::{prelude::*, utils::Duration};
use lexopt::prelude::*;

const DEFAULT_SCALE: f32 = 0.1;

#[derive(Debug)]
struct Args {
    path: PathBuf,
    scale: f32,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut parser = lexopt::Parser::from_env();
        let mut path = None;
        let mut scale = None;
        while let Some(arg) = parser.next()? {
            match arg {
                Value(v) => path = Some(v.into()),
                Long("scale") => scale = Some(parser.value()?.parse()?),
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
        })
    }
}

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .add_plugins(DefaultPlugins)
        .add_plugins(lib::MeshPlugin)
        .insert_resource(SpinTimer(Timer::from_seconds(
            1.0 / 60.0,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, setup)
        .add_systems(Update, spin_disc)
        .run();
}

#[derive(Component)]
struct Disc {
    angle: f32,
}

#[derive(Resource)]
struct SpinTimer(Timer);

fn setup(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    // mut materials: ResMut<'_, Assets<StandardMaterial>>,
) {
    let args = Args::parse().unwrap();
    eprintln!("args={args:?}");
    commands.spawn((
        {
            SceneBundle {
                scene: asset_server.load(args.path),
                ..Default::default()
            }
            // PbrBundle {
            //     mesh: asset_server.load(args.path),
            //     material: materials.add(Color::rgb(0.9, 0.4, 0.3).into()),
            //     transform: Transform::from_rotation(Quat::from_rotation_z(0.0)),
            //     ..Default::default()
            // }
        },
        Disc { angle: 0.0 },
    ));
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(30.0, 0.0, 20.0),
        point_light: PointLight {
            range: 40.0,
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(Vec3::new(0.0, -100.0, 100.0))
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

fn spin_disc(
    time: Res<'_, Time>,
    mut timer: ResMut<'_, SpinTimer>,
    mut query: Query<'_, '_, (&mut Disc, &mut Transform)>,
) {
    if timer
        .0
        .tick(Duration::from_secs_f32(time.delta_seconds()))
        .just_finished()
    {
        for (mut disc, mut transform) in &mut query {
            disc.angle += 0.3 * PI / 180.0;
            *transform = Transform::from_rotation(Quat::from_rotation_z(disc.angle));
        }
    }
}

mod lib {
    use std::{io, mem};

    use anyhow::Result;
    use bevy::{
        asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
        pbr::{PbrBundle, StandardMaterial},
        prelude::*,
        render::{
            mesh::{Indices, Mesh, VertexAttributeValues},
            render_resource::PrimitiveTopology,
        },
        scene::Scene,
        tasks::block_on,
        utils::BoxedFuture,
    };
    use bevy_ecs::world::World;

    use crate::Args;

    pub(crate) struct MeshPlugin;
    impl Plugin for MeshPlugin {
        fn build(&self, app: &mut App) {
            app.init_asset_loader::<MeshLoader>();
        }
    }

    #[derive(Default)]
    struct MeshLoader;
    impl AssetLoader for MeshLoader {
        type Asset = Scene;
        type Settings = ();
        type Error = io::Error;
        fn load<'a>(
            &'a self,
            reader: &'a mut Reader<'_>,
            _settings: &'a Self::Settings,
            load_context: &'a mut LoadContext<'_>,
        ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
            Box::pin(async move {
                let loader = mesh_loader::Loader::default().stl_parse_color(true);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).await?;
                let path = load_context.asset_path().path().to_owned();
                let scene = loader.load_from_slice_with_reader(&bytes, path, |path| {
                    let path = path.to_owned();
                    // TODO: avoid nested block_on
                    block_on(async {
                        let mut ctx = load_context.begin_labeled_asset();
                        ctx.read_asset_bytes(path)
                            .await
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    })
                })?;
                let scale = Args::parse().unwrap().scale;
                println!("{scene:?}");
                println!("{scale:?}");
                Ok(to_bevy_scene(scene, load_context, scale))
            })
        }
        fn extensions(&self) -> &[&str] {
            static EXTENSIONS: &[&str] = &["stl", "STL", "dae", "DAE", "xml", "obj", "OBJ"];
            EXTENSIONS
        }
    }

    fn to_bevy_scene(
        scene: mesh_loader::Scene,
        load_context: &mut LoadContext<'_>,
        scale: f32,
    ) -> Scene {
        let mut world = World::default();

        assert_eq!(scene.meshes.len(), scene.materials.len());
        for (mut mesh, material) in scene.meshes.into_iter().zip(scene.materials) {
            let material_handle = if material.texture.diffuse.is_some()
                || material.texture.normal.is_some()
                || material.color.diffuse.is_some()
            {
                let mut bevy_material = StandardMaterial {
                    base_color_texture: material.texture.diffuse.map(|p| load_context.load(p)),
                    normal_map_texture: material.texture.normal.map(|p| load_context.load(p)),
                    ..Default::default()
                };
                if let Some(color) = material.color.diffuse {
                    bevy_material.base_color = Color::rgb(color[0], color[1], color[2]);
                }
                Some(load_context.add_labeled_asset(material.name, bevy_material))
            } else {
                None
            };

            let mut bevy_mesh = Mesh::new(PrimitiveTopology::TriangleList);
            let num_vertices = mesh.vertices.len();
            let mut positions = Vec::with_capacity(num_vertices);
            let mut indices = Vec::with_capacity(num_vertices);
            for face in mesh.faces {
                for i in face {
                    let vertex = mesh.vertices[i as usize];
                    positions.push([vertex[0] * scale, vertex[1] * scale, vertex[2] * scale]);
                    indices.push(i);
                }
            }
            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            if mesh.texcoords[0].is_empty() {
                let uvs = vec![[0.0, 0.0]; num_vertices];
                bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            } else {
                bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, mem::take(&mut mesh.texcoords[0]));
            }
            if !mesh.colors[0].is_empty() {
                let colors: Vec<[f32; 4]> = mesh.colors[0]
                    .iter()
                    .map(|c| {
                        // TODO
                        Color::Rgba {
                            red: c[0],
                            green: c[1],
                            blue: c[2],
                            alpha: c[3],
                        }
                        .as_linear_rgba_f32()
                    })
                    .collect();
                bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            }
            bevy_mesh.set_indices(Some(Indices::U32(indices)));
            if mesh.normals.is_empty() {
                bevy_mesh.duplicate_vertices();
                bevy_mesh.compute_flat_normals();
            } else {
                bevy_mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    VertexAttributeValues::Float32x3(mesh.normals),
                );
            }
            let mesh_handle = load_context.add_labeled_asset(mesh.name, bevy_mesh);

            let mut pbr_bundle = PbrBundle {
                mesh: mesh_handle,
                ..Default::default()
            };
            if let Some(material_handle) = material_handle {
                pbr_bundle.material = material_handle;
            }
            world.spawn(pbr_bundle);
        }

        Scene::new(world)
    }
}
