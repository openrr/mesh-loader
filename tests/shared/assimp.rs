use std::str;

pub(crate) fn assimp_scene_to_scene(ai: &assimp::Scene<'_>) -> mesh_loader::Scene {
    let mut meshes = Vec::with_capacity(ai.num_meshes as usize);
    for mesh in ai.mesh_iter() {
        // assimp-rs's impl AsRef<str> for AiString does a similar, but it
        // might panic because assimp-rs doesn't handle the case where assimp
        // returns an out-of-range length (which probably means an error).
        let name = str::from_utf8(mesh.name.data.get(0..mesh.name.length).unwrap_or_default())
            .unwrap_or_default()
            .to_owned();
        let vertices = mesh.vertex_iter().map(|v| [v.x, v.y, v.z]).collect();
        let texcoords0 = if mesh.has_texture_coords(0) {
            mesh.texture_coords_iter(0).map(|v| [v.x, v.y]).collect()
        } else {
            vec![]
        };
        let texcoords1 = if mesh.has_texture_coords(1) {
            mesh.texture_coords_iter(1).map(|v| [v.x, v.y]).collect()
        } else {
            vec![]
        };
        // assimp-rs segfault without this null check.
        let normals = if mesh.normals.is_null() {
            vec![]
        } else {
            mesh.normal_iter().map(|v| [v.x, v.y, v.z]).collect()
        };
        let colors0 = if mesh.has_vertex_colors(0) {
            mesh.vertex_color_iter(0)
                .map(|v| [v.r, v.g, v.b, v.a])
                .collect()
        } else {
            vec![]
        };
        let colors1 = if mesh.has_vertex_colors(1) {
            mesh.vertex_color_iter(1)
                .map(|v| [v.r, v.g, v.b, v.a])
                .collect()
        } else {
            vec![]
        };
        let faces = mesh
            .face_iter()
            .filter_map(|f| {
                if f.num_indices == 3 {
                    Some([f[0], f[1], f[2]])
                } else {
                    assert!(f.num_indices < 3, "should be triangulated");
                    None
                }
            })
            .collect();

        let mut mesh = mesh_loader::Mesh::default();
        mesh.name = name;
        mesh.vertices = vertices;
        mesh.texcoords[0] = texcoords0;
        mesh.texcoords[1] = texcoords1;
        mesh.normals = normals;
        mesh.faces = faces;
        mesh.colors[0] = colors0;
        mesh.colors[1] = colors1;
        meshes.push(mesh);
    }
    let mut scene = mesh_loader::Scene::default();
    scene.materials = (0..meshes.len())
        .map(|_| mesh_loader::Material::default())
        .collect(); // TODO
    scene.meshes = meshes;
    scene
}
