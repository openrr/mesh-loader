use super::*;
use crate::common;

pub(super) fn build_meshes(doc: &Document) -> Vec<common::Mesh> {
    let mut meshes = Vec::with_capacity(doc.library_geometries.geometries.len());

    for mesh_ref in doc.meshes() {
        let mut mesh = common::Mesh {
            name: mesh_ref.xml.id.clone(),
            ..common::Mesh::default()
        };

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
