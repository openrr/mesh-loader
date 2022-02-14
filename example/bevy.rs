use std::{f32::consts::PI, path::PathBuf, str::FromStr};

use bevy::prelude::*;
use bevy_utils::Duration;
use clap::Clap;

use crate::lib::StlPlugin;

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

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(StlPlugin)
        .insert_resource(SpinTimer(Timer::from_seconds(1.0 / 60.0, true)))
        .add_startup_system(setup.system())
        .add_system(spin_disc.system())
        .run();
}

struct Disc {
    angle: f32,
}

struct SpinTimer(Timer);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let args = Args::parse();
    debug!(?args);
    commands
        .spawn_bundle({
            let mesh = asset_server.load(args.path);
            PbrBundle {
                mesh,
                material: materials.add(
                    Color::WHITE.into(), // rgb(material_color[0], material_color[1], material_color[2]).into(),
                ),
                transform: Transform::from_rotation(Quat::from_rotation_z(0.0)),
                ..Default::default()
            }
        })
        .insert(Disc { angle: 0.0 });
    commands.spawn_bundle(LightBundle {
        transform: Transform::from_xyz(30.0, 0.0, 20.0),
        light: Light { range: 40.0, ..Default::default() },
        ..Default::default()
    });
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_translation(Vec3::new(0.0, -100.0, 100.0))
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

fn spin_disc(
    time: Res<Time>,
    mut timer: ResMut<SpinTimer>,
    mut query: Query<(&mut Disc, &mut Transform)>,
) {
    if timer.0.tick(Duration::from_secs_f32(time.delta_seconds())).just_finished() {
        for (mut disc, mut transform) in query.iter_mut() {
            disc.angle += 0.3 * PI / 180.0;
            *transform = Transform::from_rotation(Quat::from_rotation_z(disc.angle));
        }
    }
}

mod lib {
    use std::io;

    use bevy_app::prelude::*;
    use bevy_asset::{AddAsset, AssetLoader, LoadContext, LoadedAsset};
    use bevy_render::{
        mesh::{Indices, Mesh, VertexAttributeValues},
        pipeline::PrimitiveTopology,
    };
    use bevy_utils::BoxedFuture;

    pub struct StlPlugin;

    impl Plugin for StlPlugin {
        fn build(&self, app: &mut AppBuilder) {
            app.init_asset_loader::<StlLoader>();
        }
    }

    #[derive(Default)]
    struct StlLoader;

    impl AssetLoader for StlLoader {
        fn load<'a>(
            &'a self,
            bytes: &'a [u8],
            load_context: &'a mut LoadContext,
        ) -> BoxedFuture<'a, anyhow::Result<()>> {
            Box::pin(async move { Ok(load_stl(bytes, load_context).await?) })
        }

        fn extensions(&self) -> &[&str] {
            static EXTENSIONS: &[&str] = &["stl", "STL"];
            EXTENSIONS
        }
    }

    async fn load_stl<'a, 'b>(
        bytes: &'a [u8],
        load_context: &'a mut LoadContext<'b>,
    ) -> io::Result<()> {
        let stl = mesh_loader::stl::from_slice(bytes)?;

        load_context.set_default_asset(LoadedAsset::new(stl_to_triangle_mesh(&stl, 0.2)));

        #[cfg(feature = "wireframe")]
        load_context.set_labeled_asset("wireframe", LoadedAsset::new(stl_to_wireframe_mesh(&stl)));

        Ok(())
    }

    fn stl_to_triangle_mesh(stl: &mesh_loader::stl::IndexMesh, scale: f32) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let vertex_count = stl.triangles.len() * 3;

        let mut positions = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);
        let mut indices = Vec::with_capacity(vertex_count);
        // let has_color = !stl.colors.is_empty();
        // let mut colors = Vec::with_capacity(vertex_count);
        // let default_color = Color::default(); //new(0.9, 0.4, 0.3, 0.0);
        // *MATERIAL_COLOR.lock().unwrap() = stl.material_color;

        for (i, face) in stl.triangles.iter().enumerate() {
            for j in 0..3 {
                let vertex = stl.vertices[face.vertices_indices[j]];
                positions.push([vertex[0] * scale, vertex[1] * scale, vertex[2] * scale]);
                normals.push(face.normal);
                indices.push((i * 3 + j) as u32);
                // if has_color {
                //     let color = stl.colors[face.vertices_indices[j]].unwrap_or(default_color);
                //     colors.push(color.into());
                // }
            }
        }

        let uvs = vec![[0.0, 0.0, 0.0]; vertex_count];

        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float3(positions));
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::Float3(normals));
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float3(uvs));
        // if has_color {
        //     mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, VertexAttributeValues::Float4(colors));
        // }
        mesh.set_indices(Some(Indices::U32(indices)));

        mesh
    }

    #[cfg(feature = "wireframe")]
    fn stl_to_wireframe_mesh(stl: &stl_io::IndexedMesh) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);

        let positions = stl.vertices.iter().map(|v| [v[0], v[1], v[2]]).collect();
        let mut indices = Vec::with_capacity(stl.faces.len() * 3);
        let normals = vec![[1.0, 0.0, 0.0]; stl.vertices.len()];
        let uvs = vec![[0.0, 0.0, 0.0]; stl.vertices.len()];

        for face in &stl.faces {
            for j in 0..3 {
                indices.push(face.vertices[j] as u32);
                indices.push(face.vertices[(j + 1) % 3] as u32);
            }
        }

        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float3(positions));
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::Float3(normals));
        mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float3(uvs));
        mesh.set_indices(Some(Indices::U32(indices)));

        mesh
    }
}
