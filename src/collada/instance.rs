use std::path::PathBuf;

use super::*;
use crate::ShadingModel;

pub(super) fn build(doc: &mut Document<'_>, dir: Option<&Path>) -> common::Scene {
    let mut meshes = Vec::with_capacity(doc.library_geometries.geometries.len());
    let mut materials = Vec::with_capacity(doc.library_geometries.geometries.len());
    let mut instance_geometry_map = HashMap::new();
    let mut instance_material_map = HashMap::new();

    if let Some(root) = &doc.scene.instance_visual_scene {
        if let Some(root) = doc
            .library_visual_scenes
            .nodes
            .iter_mut()
            .find(|n| n.id == Some(root.url.as_str()))
        {
            root.transform *= Matrix4x4::new(
                doc.asset.unit,
                0.,
                0.,
                0.,
                0.,
                doc.asset.unit,
                0.,
                0.,
                0.,
                0.,
                doc.asset.unit,
                0.,
                0.,
                0.,
                0.,
                1.,
            );
        }
    }

    for node in &doc.library_visual_scenes.nodes {
        for instance_geometry in &node.instance_geometry {
            let mut transform = node.transform;
            let mut parent = node.parent;
            while let Some(p) = parent {
                let node = &doc.library_visual_scenes.nodes[p];
                transform *= node.transform;
                parent = node.parent;
            }
            let transform = if transform.is_identity() {
                None
            } else {
                Some(transform)
            };
            instance_geometry_map.insert(instance_geometry.url.as_str(), transform);
            for (symbol, instance_material) in &instance_geometry.materials {
                match doc.get(&instance_material.target) {
                    Some(material) => {
                        match doc.get(&material.instance_effect.url) {
                            Some(effect) => {
                                instance_material_map.insert(*symbol, (material.name, effect));
                            }
                            None => {
                                // debug!(
                                //     "not found effect instance '{}'",
                                //     material.instance_effect.url.as_str()
                                // );
                            }
                        }
                    }
                    None => {
                        // debug!(
                        //     "not found material instance '{}'",
                        //     instance_material.target.as_str()
                        // );
                    }
                }
            }
        }
    }
    for geometry in doc.library_geometries.geometries.values() {
        if let Some(&transform) = instance_geometry_map.get(geometry.id) {
            meshes.push(build_mesh(doc, geometry, transform));
        } else {
            meshes.push(build_mesh(doc, geometry, None));
        }
        let mut material = None;
        for mat in geometry.mesh.primitives.iter().filter_map(|m| m.material) {
            if let Some(&(name, effect)) = instance_material_map.get(mat) {
                material = Some(build_material(doc, name, effect, dir));
                // TODO: multiple materials from geometry.mesh.primitives?
                break;
            }
        }
        match material {
            Some(material) => materials.push(material),
            None => materials.push(common::Material::default()),
        }
    }

    common::Scene { materials, meshes }
}

fn build_mesh(
    doc: &Document<'_>,
    geometry: &Geometry<'_>,
    transform: Option<Matrix4x4>,
) -> common::Mesh {
    let mut mesh = common::Mesh {
        name: geometry.id.to_owned(),
        ..Default::default()
    };

    for prim in (iter::Mesh { doc, xml: geometry }).primitives() {
        #[allow(clippy::cast_possible_truncation)]
        let prev_positions_len = mesh.vertices.len() as u32;
        let p: Vec<_> = prim.positions().collect();
        let n: Vec<_> = prim.normals().collect();
        let t: Vec<_> = prim.texcoords(0).collect();
        let c: Vec<_> = prim.colors().collect();
        let positions_indices = prim.vertex_indices();
        let mut normal_indices = prim.normal_indices();
        let mut texcoord_indices = prim.texcoord_indices(0);
        let mut color_indices = prim.color_indices();
        let mut idx = 0;

        for vertex_idx in positions_indices {
            for vertex_idx in vertex_idx {
                let mut v = [
                    p[vertex_idx as usize][0],
                    p[vertex_idx as usize][1],
                    p[vertex_idx as usize][2],
                ];
                if let Some(transform) = transform {
                    v *= transform;
                }
                mesh.vertices.push(v);
            }
            if !n.is_empty() {
                if let Some(normal_idx) = normal_indices.next() {
                    for normal_idx in normal_idx {
                        mesh.normals.push([
                            n[normal_idx as usize][0],
                            n[normal_idx as usize][1],
                            n[normal_idx as usize][2],
                        ]);
                    }
                } else {
                    panic!()
                }
            }
            if !t.is_empty() {
                if let Some(texcoord_idx) = texcoord_indices.next() {
                    for texcoord_idx in texcoord_idx {
                        mesh.texcoords[0]
                            .push([t[texcoord_idx as usize][0], t[texcoord_idx as usize][1]]);
                    }
                } else {
                    panic!()
                }
            }
            if !c.is_empty() {
                if let Some(rgb_idx) = color_indices.next() {
                    for rgb_idx in rgb_idx {
                        mesh.colors[0].push([
                            c[rgb_idx as usize][0],
                            c[rgb_idx as usize][1],
                            c[rgb_idx as usize][2],
                            1.,
                        ]);
                    }
                } else {
                    panic!()
                }
            }
            mesh.faces.push([
                prev_positions_len + idx,
                prev_positions_len + (idx + 1),
                prev_positions_len + (idx + 2),
            ]);
            idx += 3;
        }
    }

    mesh
}

fn build_material(
    doc: &Document<'_>,
    name: Option<&str>,
    effect: &Effect<'_>,
    dir: Option<&Path>,
) -> common::Material {
    fn texture(
        doc: &Document<'_>,
        effect: &Effect<'_>,
        texture: &Texture<'_>,
        dir: Option<&Path>,
    ) -> Option<PathBuf> {
        if texture.texture.is_empty() {
            return None;
        }
        let mut image = None;
        if let Some(sampler) = effect.profile.samplers.get(texture.texture) {
            if let Some(surface) = effect.profile.surfaces.get(sampler.source) {
                if let Some(i) = doc.get(&surface.init_from) {
                    image = Some(i);
                }
            }
        }
        if image.is_none() {
            if let Some(i) = doc.library_images.images.get(&texture.texture) {
                image = Some(i);
            }
        }
        if let Some(image) = image {
            match &image.source {
                ImageSource::Data(_data) => {} // TODO
                ImageSource::InitFrom(mut p) => {
                    // There is an exporter writes empty <init_from/> tag
                    if p.is_empty() {
                        return None;
                    }
                    match dir {
                        Some(dir) => {
                            // TODO
                            p = p.strip_prefix("file://").unwrap_or(p);
                            let tmp: String;
                            if p.contains('\\') {
                                tmp = p.replace('\\', "/");
                                p = &*tmp;
                            }
                            if p.starts_with("/..") {
                                p = p.strip_prefix('/').unwrap_or(p);
                            }
                            let p = dir.join(p);
                            if p.exists() {
                                return Some(p);
                            }
                        }
                        None => return Some(p.into()),
                    }
                }
                ImageSource::Skip => {}
            }
        }
        None
    }

    let mut mat = common::Material::default();

    if let Some(name) = name {
        mat.name = name.to_owned();
    }

    mat.shading_model = match effect.profile.technique.ty {
        _ if effect.profile.technique.faceted => Some(ShadingModel::Flat),
        ShadeType::Constant => Some(ShadingModel::NoShading),
        ShadeType::Lambert => Some(ShadingModel::Gouraud),
        ShadeType::Blinn => Some(ShadingModel::Blinn),
        ShadeType::Phong => Some(ShadingModel::Phong),
    };

    // mat.two_sided = Some(effect.profile.technique.double_sided);
    // mat.wireframe = Some(effect.profile.technique.wireframe);

    mat.color.ambient = Some(effect.profile.technique.ambient.color);
    mat.color.diffuse = Some(effect.profile.technique.diffuse.color);
    mat.color.specular = Some(effect.profile.technique.specular.color);
    mat.color.emissive = Some(effect.profile.technique.emission.color);
    mat.color.reflective = Some(effect.profile.technique.reflective.color);

    mat.shininess = Some(effect.profile.technique.shininess);
    mat.reflectivity = Some(effect.profile.technique.reflectivity);
    mat.index_of_refraction = Some(effect.profile.technique.index_of_refraction);

    // Refs: https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/Collada/ColladaLoader.cpp#L1588
    let mut transparency = effect.profile.technique.transparency;
    let mut transparent = effect.profile.technique.transparent.color;
    if transparency >= 0. && transparency <= 1. {
        if effect.profile.technique.rgb_transparency {
            transparency *=
                0.212671 * transparent[0] + 0.715160 * transparent[1] + 0.072169 * transparent[2];
            transparent[3] = 1.;
            mat.color.transparent = Some(transparent);
        } else {
            transparency *= transparent[3];
        }
        if effect.profile.technique.invert_transparency {
            transparency = 1. - transparency;
        }
        if effect.profile.technique.has_transparency || transparency < 1. {
            mat.opacity = Some(transparency);
        }
    }

    if let Some(p) = texture(doc, effect, &effect.profile.technique.ambient.texture, dir) {
        // Refs: https://github.com/assimp/assimp/blob/v5.3.1/code/AssetLib/Collada/ColladaLoader.cpp#L1619
        mat.texture.lightmap = Some(p);
    }
    if let Some(p) = texture(doc, effect, &effect.profile.technique.emission.texture, dir) {
        mat.texture.emissive = Some(p);
    }
    if let Some(p) = texture(doc, effect, &effect.profile.technique.specular.texture, dir) {
        mat.texture.specular = Some(p);
    }
    if let Some(p) = texture(doc, effect, &effect.profile.technique.diffuse.texture, dir) {
        mat.texture.diffuse = Some(p);
    }
    if let Some(p) = texture(doc, effect, &effect.profile.technique.bump, dir) {
        mat.texture.normal = Some(p);
    }
    if let Some(p) = texture(
        doc,
        effect,
        &effect.profile.technique.transparent.texture,
        dir,
    ) {
        mat.texture.opacity = Some(p);
    }
    if let Some(p) = texture(
        doc,
        effect,
        &effect.profile.technique.reflective.texture,
        dir,
    ) {
        mat.texture.reflection = Some(p);
    }

    mat
}
