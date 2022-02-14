use std::collections::HashMap;

use super::*;
use crate::common;

#[derive(Default)]
struct Context<'a> {
    material_index_by_name: HashMap<&'a str, usize>,
    new_mats: Vec<(Effect, common::Material)>,
    textures: Vec<common::Texture>,
}

pub fn build(doc: &Document) -> common::Scene {
    let mut cx = Context::default();
    let mut scene = common::Scene::default();

    build_materials(&mut cx, doc);

    let meshes = build_meshes(doc);

    fill_materials(&mut cx, doc, &mut scene);

    scene.meshes = meshes;
    scene.materials = cx.new_mats.into_iter().map(|(_, m)| m).collect();
    scene.textures = cx.textures;

    scene
}

pub fn build_meshes(doc: &Document) -> Vec<common::TriMesh> {
    let mut meshes = Vec::with_capacity(doc.library_geometries.geometries.len());

    for geometry in doc.library_geometries.geometries.values() {
        let mut mesh = common::TriMesh { name: geometry.id.clone(), ..common::TriMesh::default() };

        let mesh_ref = iter::Mesh { doc, index: 0, xml: geometry };
        for prim in mesh_ref.primitives() {
            let prev_positions_len = mesh.vertices.len() as u32;
            let p: Vec<_> = prim.positions().collect();
            let n: Vec<_> = prim.normals().collect();
            let t: Vec<_> = prim.texcoords(0).collect();
            let positions_idx = prim.vertex_indices();
            let mut normal_idx = prim.normal_indices();
            let mut texcoord_idx = prim.texcoord_indices(0);
            let mut idx = 0;

            for i in positions_idx {
                if let iter::Face::Triangle(vertex) = i {
                    mesh.vertices.push([
                        p[vertex[0] as usize][0],
                        p[vertex[0] as usize][1],
                        p[vertex[0] as usize][2],
                    ]);
                    mesh.vertices.push([
                        p[vertex[1] as usize][0],
                        p[vertex[1] as usize][1],
                        p[vertex[1] as usize][2],
                    ]);
                    mesh.vertices.push([
                        p[vertex[2] as usize][0],
                        p[vertex[2] as usize][1],
                        p[vertex[2] as usize][2],
                    ]);
                    if !n.is_empty() {
                        if let Some(iter::Face::Triangle(normal)) = normal_idx.next() {
                            mesh.normals.push([
                                n[normal[0] as usize][0],
                                n[normal[0] as usize][1],
                                n[normal[0] as usize][2],
                            ]);
                            mesh.normals.push([
                                n[normal[1] as usize][0],
                                n[normal[1] as usize][1],
                                n[normal[1] as usize][2],
                            ]);
                            mesh.normals.push([
                                n[normal[2] as usize][0],
                                n[normal[2] as usize][1],
                                n[normal[2] as usize][2],
                            ]);
                        } else {
                            panic!()
                        }
                    }
                    if !t.is_empty() {
                        if let Some(iter::Face::Triangle(texcoord)) = texcoord_idx.next() {
                            mesh.texcoords[0]
                                .push([t[texcoord[0] as usize][0], t[texcoord[0] as usize][1]]);
                            mesh.texcoords[0]
                                .push([t[texcoord[1] as usize][0], t[texcoord[1] as usize][1]]);
                            mesh.texcoords[0]
                                .push([t[texcoord[2] as usize][0], t[texcoord[2] as usize][1]]);
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
        }

        meshes.push(mesh);
    }

    meshes
}

/*
#[derive(Default)]
struct Context<'a> {
    material_index_by_name: HashMap<&'a str, usize>,
    new_mats: Vec<(&'a Effect, core::Material)>,
    meshes: Vec<TODO>,
    node_name_counter: usize,

    nodes: SlotMap<core::NodeIndex, core::Node>,
    root: Option<core::NodeIndex>,
}

fn build(doc: &Document) -> Result<core::Scene> {
    let mut cx = Context::default();

    if doc.scene.instance_visual_scene.is_none() {
        bail!("file came out empty. something is wrong here");
    }
    let root_node = doc.library_visual_scenes.nodes.get_index(
        doc.scene.instance_visual_scene.as_ref().unwrap().url.as_str(),
    );
    if root_node.is_none() {
        bail!("file came out empty. something is wrong here");
    }
    let root_node = root_node.unwrap();

    // reserve some storage to avoid unnecessary reallocation
    cx.new_mats.reserve(doc.library_materials.materials.len() * 2);
    cx.meshes.reserve(doc.library_geometries.geometries.len() * 2);
    // cx.cameras.reserve(parser.camera_library.len());
    // cx.lights.reserve(parser.light_library.len());

    // create the materials first, for the meshes to find
    build_materials(&mut cx, doc);

    // build the node hierarchy from it
    cx.root = Some(build_hierarchy(&mut cx, doc, root_node, root_node)?);

    // TODO
    todo!()
}
*/

fn add_texture<'a>(
    cx: &mut Context<'a>,
    mat: &mut Material,
    doc: &'a Document,
    effect: &mut Effect,
    ty: common::TextureType,
    idx: usize,
) {
    // TODO
}

/// Fills materials from the collada material definitions
fn fill_materials<'a>(cx: &mut Context<'a>, doc: &'a Document, scene: &mut common::Scene) {
    for (effect, mat) in &mut cx.new_mats {
        // resolve shading mode
        let mut shade_mode;
        if effect.profile.technique.faceted {
            shade_mode = common::ShadingMode::Flat;
        } else {
            match effect.profile.technique.ty {
                ShadeType::Constant => {
                    shade_mode = common::ShadingMode::NoShading;
                }
                ShadeType::Lambert => {
                    shade_mode = common::ShadingMode::Gouraud;
                }
                ShadeType::Blinn => {
                    shade_mode = common::ShadingMode::Blinn;
                }
                ShadeType::Phong => {
                    shade_mode = common::ShadingMode::Phong;
                }
            }
        }

        mat.properties.shading_model = Some(shade_mode);

        // double-sided?
        mat.properties.two_sided = effect.profile.technique.double_sided;

        // wire-frame?
        mat.properties.wireframe = effect.profile.technique.wireframe;

        // add material colors
        mat.properties.color_ambient = Some(effect.profile.technique.ambient.color);
        mat.properties.color_diffuse = Some(effect.profile.technique.diffuse.color);
        mat.properties.color_specular = Some(effect.profile.technique.specular.color);
        mat.properties.color_emissive = Some(effect.profile.technique.emission.color);
        mat.properties.color_reflective = Some(effect.profile.technique.reflective.color);

        // scalar properties
        mat.properties.shininess = Some(effect.profile.technique.shininess);
        mat.properties.reflectivity = Some(effect.profile.technique.reflectivity);
        mat.properties.refracti = Some(effect.profile.technique.index_of_refraction);

        // transparency, a very hard one. seemingly not all files are following the
        // specification here (1.0 transparency => completely opaque)...
        // therefore, we let the opportunity for the user to manually invert
        // the transparency if necessary and we add preliminary support for RGB_ZERO mode
        // TODO
        if (0.0..=1.0).contains(&effect.profile.technique.transparency) {
            // handle RGB transparency completely, cf Collada specs 1.5.0 pages 249 and 304
            if effect.profile.technique.transparent.opaque.map_or(false, |o| o.rgb_transparency()) {
            } else {
                effect.profile.technique.transparency =
                    effect.profile.technique.transparent.color[3];
            }

            if effect
                .profile
                .technique
                .transparent
                .opaque
                .map_or(false, |o| o.invert_transparency())
            {
                effect.profile.technique.transparency = 1.0 - effect.profile.technique.transparency;
            }

            if effect.profile.technique.transparency < 1.0 {
                mat.properties.opacity = Some(effect.profile.technique.transparency);
            }
        }

        // add textures, if given
        // TODO
        /*
         if (!effect.mTexAmbient.mName.empty()) {
            // It is merely a light-map
            AddTexture(mat, pParser, effect, effect.mTexAmbient, aiTextureType_LIGHTMAP);
        }

        if (!effect.mTexEmissive.mName.empty())
            AddTexture(mat, pParser, effect, effect.mTexEmissive, aiTextureType_EMISSIVE);

        if (!effect.mTexSpecular.mName.empty())
            AddTexture(mat, pParser, effect, effect.mTexSpecular, aiTextureType_SPECULAR);

        if (!effect.mTexDiffuse.mName.empty())
            AddTexture(mat, pParser, effect, effect.mTexDiffuse, aiTextureType_DIFFUSE);

        if (!effect.mTexBump.mName.empty())
            AddTexture(mat, pParser, effect, effect.mTexBump, aiTextureType_NORMALS);

        if (!effect.mTexTransparent.mName.empty())
            AddTexture(mat, pParser, effect, effect.mTexTransparent, aiTextureType_OPACITY);

        if (!effect.mTexReflective.mName.empty())
            AddTexture(mat, pParser, effect, effect.mTexReflective, aiTextureType_REFLECTION);
        */
    }
}

fn build_materials<'a>(cx: &mut Context<'a>, doc: &'a Document) {
    for (mat_id, material) in &doc.library_materials.materials {
        // a material is only a reference to an effect
        let effect = match doc.get(&material.instance_effect.url) {
            Some(effect) => effect,
            None => continue,
        };

        // create material
        let mut mat = common::Material::default();
        let name = if material.name.is_none() { mat_id } else { material.name.as_ref().unwrap() };
        mat.properties.name = Some(name.to_owned());

        // store the material
        cx.material_index_by_name.insert(mat_id, cx.new_mats.len());
        cx.new_mats.push((effect.clone(), mat));
    }
    // ScenePreprocessor generates a default material automatically if none is there.
    // All further code here in this loader works well without a valid material so
    // we can safely let it to ScenePreprocessor.
}

// Resolves the texture name for the given effect texture entry and loads the texture data
fn find_filename_for_effect_texture<'a>(
    doc: &'a Document,
    effect: &'a mut Effect,
    mut name: &'a str,
) -> String {
    // TODO
    if let Some(s) = effect.profile.samplers.get(name) {
        name = &*s.source;
    }

    if let Some(s) = effect.profile.surfaces.get(name) {
        name = &*s.init_from;
    }

    if let Some(i) = doc.library_images.images.get(name) {
        match &i.source {
            ImageSource::Data(..) => {
                let mut tex = common::Texture::default();
            }
            ImageSource::InitFrom(filename) => {}
        }
    }

    todo!()
}

/*
fn build_hierarchy<'a>(
    cx: &mut Context<'a>,
    doc: &'a Document,
    p_node: NodeIndex,
    root_node: NodeIndex,
) -> Result<core::NodeIndex> {
    let p_node = &doc[&p_node];
    // create a node for it
    let mut node = core::Node::default();
    let this = cx.nodes.insert(node);

    // find a name for the new node. It's more complicated than you might think
    cx.nodes[this].name = find_name_for_node(cx, p_node);

    // if we're not using the unique IDs, hold onto them for reference and export
    // if cx.use_collada_name {
    //     if !p_node.id.is_empty() {
    //         node.borrow_mut().add_node_metadata(
    //             core::METADATA_COLLADA_ID,
    //             p_node.id.to_owned(),
    //         );
    //     }
    //     if !p_node.sid.is_empty() {
    //         node.borrow_mut().add_node_metadata(
    //             core::METADATA_COLLADA_SID,
    //             p_node.sid.to_owned(),
    //         );
    //     }
    // }

    // calculate the transformation matrix for it
    // node.transformation = doc.calculate_result_transform(&p_node.transforms);

    // now resolve node instances
    let mut instances = vec![];
    resolve_node_instances(cx, doc, p_node, &mut instances, root_node);

    // add children. first the *real* ones
    cx.nodes[this].children.reserve(p_node.children.len() + instances.len());

    for &c in &p_node.children {
        let child = build_hierarchy(cx, doc, c, root_node)?;
        cx.nodes[child].parent = Some(this);
        cx.nodes[this].children.push(child);
    }

    // ... and finally the resolved node instances
    for &c in &instances {
        let child = build_hierarchy(cx, doc, c, root_node)?;
        cx.nodes[child].parent = Some(this);
        cx.nodes[this].children.push(child);
    }

    // build_meshes_for_node(cx, doc, p_node, this)?;
    // TODO
    // BuildCamerasForNode(pParser, pNode, node);
    // BuildLightsForNode(pParser, pNode, node);

    Ok(this)
}

fn resolve_node_instances<'a>(
    cx: &mut Context<'a>,
    doc: &'a Document,
    p_node: &Node,
    resolved: &mut Vec<NodeIndex>,
    root_node: NodeIndex,
) {
    resolved.reserve(p_node.node_instances.len());

    // iterate through all nodes to be instanced as children of pNode
    for node_inst in &p_node.node_instances {
        // find the corresponding node in the library
        let mut nd = doc
            .library_visual_scenes
            .nodes
            .get_index(&node_inst.node)
            .unwrap_or_default();

        // FIX for http://sourceforge.net/tracker/?func=detail&aid=3054873&group_id=226462&atid=1067632
        // need to check for both name and ID to catch all. To avoid breaking valid files,
        // the workaround is only enabled when the first attempt to resolve the node has failed.
        if nd.is_null() {
            nd = find_node(doc, root_node, &node_inst.node);
        }

        if nd.is_null() {
            error!(
                "Unable to resolve reference to instanced node {:?}",
                node_inst.node
            );
        } else {
            // attach this node to the list of children
            resolved.push(nd);
        }
    }
}

/*
fn apply_vertex_to_effect_semantic_mapping(
    sampler: &mut Sampler<'a>,
    table: &SemanticMappingTable<'a>,
) {
    if let Some(i) = table.map.get(sampler.uv_channel) {
        if i.ty != InputType::Texcoord {
            warn!("unexpected effect input mapping");
        }

        sampler.uv_id = i.set;
    }
}
*/

/*
/// Builds meshes for the given node and references them
fn build_meshes_for_node<'a>(
    cx: &mut Context<'a>,
    doc: &'a Document,
    p_node: &Node,
    target: core::NodeIndex,
) -> Result<()> {
    // accumulated mesh references by this node
    let mut new_mesh_refs = Vec::with_capacity(p_node.geometry_instances.len());

    // add a mesh for each subgroup in each collada mesh
    for mid in &p_node.geometry_instances {
        let mut src_mesh = None;
        let mut src_controller = None;

        // find the referred mesh
        if let Some(src) = doc.get(&mid.url) {
            // ID found in the mesh library -> direct reference to an unskinned mesh
            src_mesh = Some(src);
        }
        //  else {
        //     src_controller = doc.controller_library.get(mid.mesh_or_controller);
        //     if let Some(src_controller) = src_controller {
        //         // if not found in the mesh-library, it might also be a controller referring to a mesh
        //         if let Some(src) =
        //             parser.mesh_library.get(src_controller.mesh_id)
        //         {
        //             src_mesh = Some(src);
        //         }
        //     }

        //     if src_mesh.is_none() {
        //         warn!(
        //             "unable to find geometry for ID {:?}. skipping",
        //             mid.mesh_or_controller
        //         );
        //         continue;
        //     }
        // }

        // build a mesh for each of its subgroups
        let mut vertex_start = 0;
        let mut face_start = 0;
        let src_mesh = src_mesh.unwrap();
        for (i, submesh) in src_mesh.mesh.primitives.iter().enumerate() {
            if submesh.count == 0 {
                continue;
            }

            // find material assigned to this submesh
            let mut mesh_material = None;
            let mut table =
                submesh.material.as_ref().and_then(|i| mid.materials.get(i));

            if let Some(mesh_mat) = table {
                table = Some(mesh_mat);
                mesh_material = Some(&mesh_mat.target);
            } else {
                warn!(
                    "no material specified for subgroup <{}> in geometry <{}>",
                    submesh.material.as_deref().unwrap_or_default(),
                    mid.url.as_str()
                );
                if !mid.materials.is_empty() {
                    mesh_material = mid.materials.first().map(|m| &m.1.target);
                }
            }

            // OK ... here the *real* fun starts ... we have the vertex-input-to-effect-semantic-table
            // given. The only mapping stuff which we do actually support is the UV channel.
            let mut mat_idx = 0;
            if let Some(mesh_material) = mesh_material {
                if let Some(&mat) =
                    cx.material_index_by_name.get(mesh_material.as_str())
                {
                    mat_idx = mat;
                }
            }

            if let Some(table) = table {
                if !table.map.is_empty() {
                    let mat = &mut cx.new_mats[mat_idx];

                    /*
                    // Iterate through all texture channels assigned to the effect and
                    // check whether we have mapping information for it.
                    Self::apply_vertex_to_effect_semantic_mapping(
                        &mut mat.0.tex_diffuse,
                        table,
                    );
                    Self::apply_vertex_to_effect_semantic_mapping(
                        &mut mat.0.tex_ambient,
                        table,
                    );
                    Self::apply_vertex_to_effect_semantic_mapping(
                        &mut mat.0.tex_specular,
                        table,
                    );
                    Self::apply_vertex_to_effect_semantic_mapping(
                        &mut mat.0.tex_emissive,
                        table,
                    );
                    Self::apply_vertex_to_effect_semantic_mapping(
                        &mut mat.0.tex_transparent,
                        table,
                    );
                    Self::apply_vertex_to_effect_semantic_mapping(
                        &mut mat.0.tex_bump,
                        table,
                    );
                    */
                }
            }

            // built lookup index of the Mesh-Submesh-Material combination
            let index = ColladaMeshIndex {
                mesh_id: mid.mesh_or_controller.to_owned(),
                submesh: i,
                material: mesh_material.unwrap_or_default().to_owned(),
            };

            // if we already have the mesh at the library, just add its index to the node's array
            if let Some(&dst_mesh) = self.mesh_index_by_id.get(&index) {
                new_mesh_refs.push(dst_mesh);
            } else {
                if src_mesh.positions.is_empty() {
                    // TODO(taiki-e): without this, create_mesh panics:
                    // panicked at 'range end index 6 out of range for slice of length 0', src/collada/loader.rs:421:33
                    continue;
                }

                // else we have to add the mesh to the collection and store its newly assigned index at the node
                let dst_mesh = self.create_mesh(
                    parser,
                    src_mesh,
                    submesh,
                    src_controller,
                    vertex_start,
                    face_start,
                )?;

                // store the mesh, and store its new index in the node
                new_mesh_refs.push(self.meshes.len());
                self.mesh_index_by_id.insert(index, self.meshes.len());
                self.meshes.push(dst_mesh);
                let dst_mesh = &mut self.mesh_arena[dst_mesh];
                vertex_start += dst_mesh.vertices.len();
                face_start += dst_mesh.faces.len();

                // assign the material index
                if let Some(&sub_mat) =
                    self.material_index_by_name.get(submesh.material)
                {
                    dst_mesh.material_index = sub_mat as _;
                } else {
                    dst_mesh.material_index = mat_idx as _;
                }
                if dst_mesh.name.is_empty() {
                    dst_mesh.name = mid.mesh_or_controller.to_owned();
                }
            }
        }
    }

    // now place all mesh references we gathered in the target node
    if !new_mesh_refs.is_empty() {
        let target = &mut cx.nodes[target];
        target.meshes.reserve(new_mesh_refs.len());
        for n in new_mesh_refs {
            target.meshes.push(n as _);
        }
    }
    Ok(())
}
*/
/*
/// Find mesh from either meshes or morph target meshes
fn find_mesh(cx: &mut Context<'_>, mesh_id: &str) -> Option<MeshIndex> {
    if mesh_id.is_empty() {
        return None;
    }

    for &mesh in &cx.meshes {
        if cx.mesh_arena[mesh].name == mesh_id {
            return Some(mesh);
        }
    }

    for &target_mesh in &cx.meshes {
        if cx.mesh_arena[target_mesh].name == mesh_id {
            return Some(target_mesh);
        }
    }

    None
}
*/
/*
/// Creates a mesh for the given ColladaMesh face subset and returns the newly created mesh
fn create_mesh<'a>(
    cx: &mut Context<'a>,
    doc: &'a Document,
    src_mesh: &Geometry,
    sub_mesh: &Primitive,
    // src_controller: Option<&super::Controller>,
    start_vertex: usize,
    start_face: usize,
) -> Result<MeshIndex> {
    let mut dst_mesh = core::Mesh::default();

    // if self.use_collada_name {
    //     dst_mesh.name = src_mesh.name.to_owned();
    // } else {
    dst_mesh.name = src_mesh.id.to_owned();
    // }

    // count the vertices addressed by its faces
    let num_vertices = src_mesh.face_size
        [start_face..start_face + sub_mesh.num_faces]
        .iter()
        .map(|&n| n as usize)
        .sum();

    // copy positions
    dst_mesh.vertices.reserve(num_vertices);
    dst_mesh.vertices.extend_from_slice(
        &src_mesh.positions[start_vertex..start_vertex + num_vertices],
    );

    // normals, if given. HACK: (thom) Due to the glorious Collada spec we never
    // know if we have the same number of normals as there are positions. So we
    // also ignore any vertex attribute if it has a different count
    if src_mesh.normals.len() >= start_vertex + num_vertices {
        dst_mesh.normals.reserve(num_vertices);
        dst_mesh.normals.extend_from_slice(
            &src_mesh.normals[start_vertex..start_vertex + num_vertices],
        );
    }

    // tangents, if given.
    if src_mesh.tangents.len() >= start_vertex + num_vertices {
        dst_mesh.tangents.reserve(num_vertices);
        dst_mesh.tangents.extend_from_slice(
            &src_mesh.tangents[start_vertex..start_vertex + num_vertices],
        );
    }

    // bitangents, if given.
    if src_mesh.bitangents.len() >= start_vertex + num_vertices {
        dst_mesh.bitangents.reserve(num_vertices);
        dst_mesh.bitangents.extend_from_slice(
            &src_mesh.bitangents[start_vertex..start_vertex + num_vertices],
        );
    }

    // same for texture coords, as many as we have
    // empty slots are not allowed, need to pack and adjust UV indexes accordingly
    let mut real = 0;
    for i in 0..MAX_NUMBER_OF_TEXTURE_COORDS {
        if src_mesh.texcoords[i].len() >= start_vertex + num_vertices {
            dst_mesh.texture_coords[real].reserve(num_vertices);
            for j in 0..num_vertices {
                dst_mesh.texture_coords[real]
                    .push(src_mesh.texcoords[i][start_vertex + j]);
            }

            dst_mesh.uv_components[real] = src_mesh.num_uv_components[i];
            real += 1;
        }
    }

    // same for vertex colors, as many as we have. again the same packing to avoid empty slots
    let mut real = 0;
    for i in 0..MAX_NUMBER_OF_TEXTURE_COORDS {
        if src_mesh.colors[i].len() >= start_vertex + num_vertices {
            dst_mesh.colors[real].reserve(num_vertices);
            dst_mesh.colors[real].extend_from_slice(
                &src_mesh.colors[i][start_vertex..start_vertex + num_vertices],
            );
            real += 1;
        }
    }

    // create faces. Due to the fact that each face uses unique vertices, we can simply count up on each vertex
    let mut vertex = 0;
    dst_mesh.faces.reserve(sub_mesh.num_faces);
    for i in 0..sub_mesh.num_faces {
        let s = src_mesh.face_size[start_face + i] as usize;
        let mut face = core::Face::default();
        face.indices.reserve(s);
        for _ in 0..s {
            face.indices.push(vertex);
            vertex += 1;
        }
        dst_mesh.faces.push(face);
    }

    // create morph target meshes if any
    let mut target_meshes = vec![];
    let mut target_weights = vec![];
    let mut method = MorphMethod::Normalized;

    /*
    for c in parser.controller_library.values() {
        let mut base_mesh =
            resolve_library_reference(&parser.mesh_library, c.mesh_id)?;

        if c.ty == ControllerType::Morph && base_mesh.name == src_mesh.name {
            let target_accessor = resolve_library_reference3(
                &parser.accessor_library,
                c.morph_target,
            )?;
            let weight_accessor = resolve_library_reference3(
                &parser.accessor_library,
                c.morph_weight,
            )?;
            let target_data = resolve_library_reference3(
                &parser.data_library,
                target_accessor.source,
            )?;
            let weight_data = resolve_library_reference3(
                &parser.data_library,
                weight_accessor.source,
            )?;

            // take method
            method = c.method;

            let target_data = target_data
                .as_string()
                .ok_or_else(|| format_err!("target data must contain id"))?;
            let weight_data = weight_data.as_float().ok_or_else(|| {
                format_err!("target weight data must not be textual")
            })?;

            for &string in target_data {
                let target_mesh =
                    resolve_library_reference(&parser.mesh_library, string)?;

                if let Some(core_mesh) =
                    self.find_mesh(if self.use_collada_name {
                        target_mesh.name
                    } else {
                        target_mesh.id
                    })
                {
                    target_meshes.push(core_mesh);
                } else {
                    if target_mesh.sub_meshes.len() != 1 {
                        bail!("morphing target mesh must be a single");
                    }
                    let core_mesh = self.create_mesh(
                        parser,
                        target_mesh,
                        &target_mesh.sub_meshes[0],
                        None,
                        0,
                        0,
                    )?;
                    self.target_meshes.push(core_mesh);
                    target_meshes.push(core_mesh);
                }
            }

            target_weights.extend_from_slice(weight_data);
        }
    }
    */
    if !target_meshes.is_empty() && target_weights.len() == target_meshes.len()
    {
        // let mut anim_meshes = vec![];
        for i in 0..target_meshes.len() {
            let target_mesh = &target_meshes[i];
            // TODO
        }
        // TODO
    }

    // create bones if given
    if let Some(src_controller) = src_controller {
        if src_controller.ty == ControllerType::Skin {
            // TODO
        }
    }

    Ok(self.mesh_arena.insert(dst_mesh))
}
*/
fn find_node(doc: &Document, node: NodeIndex, name: &str) -> NodeIndex {
    let node_ = &doc[&node];
    if node_.name.as_deref() == Some(name) || node_.id.as_deref() == Some(name)
    {
        return node;
    }

    for a in &node_.children {
        let node = find_node(doc, *a, name);
        if !node.is_null() {
            return node;
        }
    }

    NodeIndex::default()
}

/// Finds a proper unique name for a node derived from the collada-node's properties.
/// The name must be unique for proper node-bone association.
fn find_name_for_node(cx: &mut Context<'_>, node: &Node) -> String {
    // If explicitly requested, just use the collada name.
    // if self.use_collada_name {
    //     if !node.name.is_empty() {
    //         node.name.to_owned()
    //     } else {
    //         self.node_name_counter += 1;
    //         format!("$ColladaAutoName$_{}", self.node_name_counter)
    //     }
    // } else {
    // Now setup the name of the assimp node. The collada name might not be
    // unique, so we use the collada ID.
    if let Some(id) = &node.id {
        id.clone()
    } else if let Some(sid) = &node.sid {
        sid.clone()
    } else {
        // No need to worry. Unnamed nodes are no problem at all, except
        // if cameras or lights need to be assigned to them.
        cx.node_name_counter += 1;
        format!("$ColladaAutoName$_{}", cx.node_name_counter)
    }
    // }
}
*/
