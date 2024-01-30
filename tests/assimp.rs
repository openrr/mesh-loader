#![allow(
    clippy::match_same_arms, // https://github.com/rust-lang/rust-clippy/issues/12044
)]

use std::{collections::BTreeSet, ffi::OsStr, path::Path, str};

use anyhow::Result;
use duct::cmd;
use fs_err as fs;
use walkdir::WalkDir;

#[test]
fn test() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assimp_dir = &manifest_dir.join("tests/fixtures/assimp");

    clone(assimp_dir, "assimp/assimp", &["/test/models/"]).unwrap();
    let models = &assimp_dir.join("test/models");

    let mut collada_models = BTreeSet::new();
    let mut obj_models = BTreeSet::new();
    let mut stl_models = BTreeSet::new();
    for e in WalkDir::new(models).into_iter().filter_map(Result::ok) {
        let path = e.path();
        match path.extension().and_then(OsStr::to_str) {
            Some("dae" | "DAE") => collada_models.insert(path.to_owned()),
            Some("obj" | "OBJ") => obj_models.insert(path.to_owned()),
            Some("stl" | "STL") => stl_models.insert(path.to_owned()),
            ext => match path.parent().unwrap().file_stem().and_then(OsStr::to_str) {
                Some("Collada") if ext == Some("xml") => collada_models.insert(path.to_owned()),
                Some("STL") => stl_models.insert(path.to_owned()),
                _ => false,
            },
        };
    }
    assert_eq!(collada_models.len(), 26);
    assert_eq!(obj_models.len(), 26);
    assert_eq!(stl_models.len(), 9);

    let mesh_loader = mesh_loader::Loader::default().stl_parse_color(true);
    let mut assimp_importer = assimp::Importer::new();
    assimp_importer.pre_transform_vertices(|x| x.enable = true);
    assimp_importer.collada_ignore_up_direction(true);
    assimp_importer.triangulate(true);

    // COLLADA
    for path in &collada_models {
        eprintln!();
        eprintln!("parsing {:?}", path.strip_prefix(manifest_dir).unwrap());
        let filename = path.file_name().unwrap().to_str().unwrap();

        // mesh-loader
        let ml = mesh_loader.load(path).unwrap();
        for (i, m) in ml.meshes.iter().enumerate() {
            eprintln!("ml.meshes[{i}]={m:?}");
        }
        let ml = &mesh_loader::Mesh::merge(ml.meshes);
        eprintln!("merge(ml.meshes)={ml:?}");
        // assert_ne!(ml.vertices.len(), 0);
        assert_eq!(ml.vertices.len(), ml.faces.len() * 3);
        if ml.normals.is_empty() {
            assert_eq!(ml.normals.capacity(), 0);
        } else {
            assert_eq!(ml.vertices.len(), ml.normals.len());
        }
        for texcoords in &ml.texcoords {
            if texcoords.is_empty() {
                assert_eq!(texcoords.capacity(), 0);
            } else {
                assert_eq!(ml.vertices.len(), texcoords.len());
            }
        }
        for colors in &ml.colors {
            if colors.is_empty() {
                assert_eq!(colors.capacity(), 0);
            } else {
                assert_eq!(ml.vertices.len(), colors.len());
            }
        }

        // assimp
        match filename {
            // assimp parse error: Cannot parse string \"  0.0 0.0 0.0 1.0  \" as a real number: does not start with digit or decimal point followed by digit.
            "library_animation_clips.dae" => continue,
            // assimp error: "Collada: File came out empty. Something is wrong here."
            "cube_tristrips.dae" | "cube_UTF16LE.dae" if option_env!("CI").is_some() => continue,
            _ => {}
        }
        let ai = assimp_importer.read_file(path.to_str().unwrap()).unwrap();
        let ai = &merge_assimp_meshes(&ai);

        // TODO
        if !matches!(
            filename,
            "ConcavePolygon.dae" | "cameras.dae" | "lights.dae" | "teapot_instancenodes.DAE"
        ) {
            assert_eq!(ml.faces.len(), ai.faces.len());
            // TODO
            if !matches!(
                filename,
                "AsXML.xml"
                    | "anims_with_full_rotations_between_keys.DAE"
                    | "Cinema4D.dae"
                    | "COLLADA.dae"
                    | "cube_emptyTags.dae"
                    | "cube_UTF16LE.dae"
                    | "cube_UTF8BOM.dae"
                    | "cube_xmlspecialchars.dae"
                    | "duck.dae"
                    | "sphere.dae"
                    | "teapots.DAE"
            ) {
                assert_faces(ml, ai);
            }
        }
        // TODO
        if !matches!(
            filename,
            "AsXML.xml"
                | "anims_with_full_rotations_between_keys.DAE"
                | "cameras.dae"
                | "COLLADA.dae"
                | "ConcavePolygon.dae"
                | "cube_emptyTags.dae"
                | "cube_UTF16LE.dae"
                | "cube_UTF8BOM.dae"
                | "cube_xmlspecialchars.dae"
                | "duck.dae"
                | "lights.dae"
                | "sphere.dae"
                | "teapot_instancenodes.DAE"
        ) {
            assert_eq!(ml.vertices.len(), ai.vertices.len());
            assert_eq!(ml.normals.len(), ai.normals.len());
            // TODO
            if !matches!(
                filename,
                "box_nested_animation.dae"
                    | "Cinema4D.dae"
                    | "cube_tristrips.dae"
                    | "cube_with_2UVs.DAE"
                    | "earthCylindrical.DAE"
                    | "kwxport_test_vcolors.dae"
                    | "regr01.dae"
                    | "teapots.DAE"
            ) {
                assert_vertices(ml, ai, f32::EPSILON * 1000.);
            }
            if !matches!(
                filename,
                "Cinema4D.dae"
                    | "cube_tristrips.dae"
                    | "earthCylindrical.DAE"
                    | "kwxport_test_vcolors.dae"
                    | "regr01.dae"
                    | "teapots.DAE"
            ) {
                assert_normals(ml, ai, f32::EPSILON * 10.);
            }
            if !matches!(
                filename,
                "Cinema4D.dae" | "earthCylindrical.DAE" | "regr01.dae" | "teapots.DAE"
            ) {
                assert_texcoords0(ml, ai, f32::EPSILON);
            }
            if !matches!(filename, "Cinema4D.dae" | "kwxport_test_vcolors.dae") {
                assert_colors0(ml, ai, f32::EPSILON);
            }
        }
    }

    // OBJ
    for path in &obj_models {
        eprintln!();
        eprintln!("parsing {:?}", path.strip_prefix(manifest_dir).unwrap());
        let filename = path.file_name().unwrap().to_str().unwrap();

        // mesh-loader
        if path.parent().unwrap().file_name().unwrap() == "invalid"
            && !matches!(filename, "malformed2.obj")
            || matches!(filename, "point_cloud.obj" | "number_formats.obj")
        {
            if matches!(filename, "point_cloud.obj" | "empty.obj") {
                // TODO: should not be allowed
                let _s = mesh_loader.load(path).unwrap();
            } else {
                let _e = mesh_loader.load(path).unwrap_err();
            }
            // TODO: assimp accepts number format that mesh-loader doesn't accept.
            if matches!(filename, "number_formats.obj")
                || matches!(filename, "point_cloud.obj") && option_env!("CI").is_some()
            {
                let _s = assimp_importer.read_file(path.to_str().unwrap()).unwrap();
            } else {
                let _e = assimp_importer
                    .read_file(path.to_str().unwrap())
                    .err()
                    .unwrap();
            }
            continue;
        }
        let ml = mesh_loader.load(path).unwrap();
        for (i, m) in ml.meshes.iter().enumerate() {
            eprintln!("ml.meshes[{i}]={m:?}");
        }
        let ml = &mesh_loader::Mesh::merge(ml.meshes);
        eprintln!("merge(ml.meshes)={ml:?}");
        if matches!(filename, "testline.obj" | "testpoints.obj") {
            assert_eq!(ml.vertices.len(), 0);
        } else {
            assert_ne!(ml.vertices.len(), 0);
        }
        assert_eq!(ml.vertices.len(), ml.faces.len() * 3);
        if ml.normals.is_empty() {
            // assert_eq!(ml.normals.capacity(), 0);
        } else {
            assert_eq!(ml.vertices.len(), ml.normals.len());
        }
        for texcoords in &ml.texcoords {
            if texcoords.is_empty() {
                assert_eq!(texcoords.capacity(), 0);
            } else {
                assert_eq!(ml.vertices.len(), texcoords.len());
            }
        }
        for colors in &ml.colors {
            if colors.is_empty() {
                assert_eq!(colors.capacity(), 0);
            } else {
                assert_eq!(ml.vertices.len(), colors.len());
            }
        }

        // assimp
        match filename {
            // Less faces loaded only in CI...
            "cube_with_vertexcolors.obj" | "cube_with_vertexcolors_uni.obj"
                if option_env!("CI").is_some() =>
            {
                continue
            }
            _ => {}
        }
        let ai = assimp_importer.read_file(path.to_str().unwrap()).unwrap();
        let ai = &merge_assimp_meshes(&ai);

        // TODO
        if !matches!(
            filename,
            "box.obj"
                | "box_UTF16BE.obj"
                | "box_longline.obj"
                | "box_mat_with_spaces.obj"
                | "box_without_lineending.obj"
                | "concave_polygon.obj"
                | "cube_usemtl.obj"
                | "multiple_spaces.obj"
                | "only_a_part_of_vertexcolors.obj"
                | "regr_3429812.obj"
                | "regr01.obj"
                | "space_in_material_name.obj"
                | "spider.obj"
                | "testmixed.obj"
        ) {
            assert_eq!(ml.faces.len(), ai.faces.len());
            if !matches!(filename, "malformed2.obj") {
                assert_faces(ml, ai);
                if !matches!(filename, "testline.obj" | "testpoints.obj") {
                    assert_eq!(ml.vertices.len(), ai.vertices.len());
                    if !matches!(filename, "cube_usemtl.obj") {
                        assert_vertices(ml, ai, f32::EPSILON * 10.);
                    }
                }
            }
            assert_normals(ml, ai, f32::EPSILON);
            assert_texcoords0(ml, ai, f32::EPSILON);
            assert_colors0(ml, ai, f32::EPSILON);
        }
    }

    // STL
    for path in &stl_models {
        eprintln!();
        eprintln!("parsing {:?}", path.strip_prefix(manifest_dir).unwrap());
        let filename = path.file_name().unwrap().to_str().unwrap();

        // mesh-loader
        let ml = mesh_loader.load(path).unwrap();
        for (i, m) in ml.meshes.iter().enumerate() {
            eprintln!("ml.meshes[{i}]={m:?}");
        }
        let ml = &mesh_loader::Mesh::merge(ml.meshes);
        eprintln!("merge(ml.meshes)={ml:?}");
        assert_ne!(ml.vertices.len(), 0);
        assert_eq!(ml.vertices.len(), ml.faces.len() * 3);
        assert_eq!(ml.vertices.len(), ml.normals.len());
        for texcoords in &ml.texcoords {
            assert_eq!(texcoords.len(), 0);
            assert_eq!(texcoords.capacity(), 0);
        }
        for (i, colors) in ml.colors.iter().enumerate() {
            if i != 0 {
                assert_eq!(colors.len(), 0);
                assert_eq!(colors.capacity(), 0);
            } else if colors.is_empty() {
                assert_eq!(colors.capacity(), 0);
            } else {
                assert_eq!(ml.vertices.len(), colors.len());
            }
        }

        // assimp
        match filename {
            // assimp error: "STL: ASCII file is empty or invalid; no data loaded"
            "triangle_with_empty_solid.stl" if option_env!("CI").is_some() => continue,
            _ => {}
        }
        let ai = assimp_importer.read_file(path.to_str().unwrap()).unwrap();
        assert_eq!(ai.num_meshes, 1);
        assert_eq!(ai.num_meshes, ai.num_materials);
        {
            let ai = ai.mesh(0).unwrap();
            assert_eq!(ai.num_vertices, ai.num_faces * 3);
            assert_eq!(ai.num_vertices as usize, ai.vertex_iter().count());
            assert_eq!(ai.num_vertices as usize, ai.normal_iter().count());
            assert!(!ai.has_texture_coords(0));
            if ai.has_vertex_colors(0) {
                assert_eq!(ai.num_vertices as usize, ai.vertex_color_iter(0).count());
            }
            assert!(!ai.has_texture_coords(1));
        }
        let ai = &merge_assimp_meshes(&ai);

        assert_faces(ml, ai);
        assert_vertices(ml, ai, f32::EPSILON * 10.);
        assert_normals(ml, ai, f32::EPSILON);
        assert_texcoords0(ml, ai, f32::EPSILON);
        assert_colors0(ml, ai, f32::EPSILON);
    }
}

fn merge_assimp_meshes(ai: &assimp::Scene<'_>) -> mesh_loader::Mesh {
    println!(
        "ai.num_meshes={},ai.num_materials={}",
        ai.num_meshes, ai.num_materials
    );
    let mut vertices = vec![];
    let mut texcoords0 = vec![];
    let mut normals = vec![];
    let mut faces = vec![];
    let mut colors0 = vec![];
    for mesh in ai.mesh_iter() {
        #[allow(clippy::cast_possible_truncation)]
        let last = vertices.len() as u32;
        vertices.extend(mesh.vertex_iter().map(|v| [v.x, v.y, v.z]));
        if mesh.has_texture_coords(0) {
            texcoords0.extend(mesh.texture_coords_iter(0).map(|v| [v.x, v.y]));
        }
        // assimp-rs segfault without this null check.
        if !mesh.normals.is_null() {
            normals.extend(mesh.normal_iter().map(|v| [v.x, v.y, v.z]));
        }
        if mesh.has_vertex_colors(0) {
            colors0.extend(mesh.vertex_color_iter(0).map(|v| [v.r, v.g, v.b, v.a]));
        }
        faces.extend(mesh.face_iter().filter_map(|f| {
            if f.num_indices == 3 {
                Some([f[0] + last, f[1] + last, f[2] + last])
            } else {
                assert!(f.num_indices < 3, "should be triangulated");
                None
            }
        }));
    }
    let mut mesh = mesh_loader::Mesh::default();
    mesh.vertices = vertices;
    mesh.texcoords[0] = texcoords0;
    mesh.normals = normals;
    mesh.faces = faces;
    mesh.colors[0] = colors0;
    mesh
}

#[track_caller]
fn assert_faces(ml: &mesh_loader::Mesh, ai: &mesh_loader::Mesh) {
    assert_eq!(ml.faces.len(), ai.faces.len());
    for (i, (ml, ai)) in ml.faces.iter().zip(&ai.faces).enumerate() {
        assert_eq!(ml, ai, "faces[{i}]");
    }
}
#[track_caller]
fn assert_vertices(ml: &mesh_loader::Mesh, ai: &mesh_loader::Mesh, eps: f32) {
    assert_eq!(ml.vertices.len(), ai.vertices.len());
    for (i, (ml, ai)) in ml.vertices.iter().zip(&ai.vertices).enumerate() {
        for j in 0..ml.len() {
            let (a, b) = (ml[j], ai[j]);
            assert!(
                (a - b).abs() < eps,
                "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at vertices[{i}][{j}]",
                (a - b).abs()
            );
        }
    }
}
#[track_caller]
fn assert_normals(ml: &mesh_loader::Mesh, ai: &mesh_loader::Mesh, eps: f32) {
    assert_eq!(ml.normals.len(), ai.normals.len());
    for (i, (ml, ai)) in ml.normals.iter().zip(&ai.normals).enumerate() {
        for j in 0..ml.len() {
            let (a, b) = (ml[j], ai[j]);
            assert!(
                (a - b).abs() < eps,
                "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at normals[{i}][{j}]",
                (a - b).abs()
            );
        }
    }
}
#[track_caller]
fn assert_texcoords0(ml: &mesh_loader::Mesh, ai: &mesh_loader::Mesh, eps: f32) {
    assert_eq!(ml.texcoords[0].len(), ai.texcoords[0].len());
    for (i, (ml, ai)) in ml.texcoords[0].iter().zip(&ai.texcoords[0]).enumerate() {
        for j in 0..ml.len() {
            let (a, b) = (ml[j], ai[j]);
            assert!(
                (a - b).abs() < eps,
                "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at texcoords[0][{i}][{j}]",
                (a - b).abs()
            );
        }
    }
}
#[track_caller]
fn assert_colors0(ml: &mesh_loader::Mesh, ai: &mesh_loader::Mesh, eps: f32) {
    assert_eq!(ml.colors[0].len(), ai.colors[0].len());
    for (i, (ml, ai)) in ml.colors[0].iter().zip(&ai.colors[0]).enumerate() {
        for j in 0..ml.len() {
            let (a, b) = (ml[j], ai[j]);
            assert!(
                (a - b).abs() < eps,
                "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at colors[0][{i}][{j}]",
                (a - b).abs()
            );
            assert!(a >= 0. && a <= 100.);
        }
    }
}

#[track_caller]
fn clone(src_dir: &Path, repository: &str, sparse_checkout: &[&str]) -> Result<()> {
    assert!(!repository.is_empty());
    assert!(!sparse_checkout.is_empty());
    let name = repository.strip_suffix(".git").unwrap_or(repository);
    assert!(!name.contains("://"), "{}", name);
    let repository = if repository.contains("://") {
        repository.to_owned()
    } else {
        format!("https://github.com/{repository}.git")
    };
    if !src_dir.exists() {
        fs::create_dir_all(src_dir.parent().unwrap())?;
        cmd!(
            "git",
            "clone",
            "--depth",
            "1",
            "--filter=tree:0",
            "--no-checkout",
            repository,
            &src_dir,
        )
        .run()?;
    }
    cmd!("git", "sparse-checkout", "init").dir(src_dir).run()?;
    let mut out = String::from("/*\n!/*/\n"); // always download top-level files
    out.push_str(&sparse_checkout.join("\n"));
    fs::write(src_dir.join(".git/info/sparse-checkout"), out)?;
    cmd!("git", "checkout")
        .dir(src_dir)
        .stdout_capture()
        .run()?;
    cmd!("git", "clean", "-df")
        .dir(src_dir)
        .stdout_capture()
        .run()?;
    // TODO: use stash?
    cmd!("git", "checkout", ".")
        .dir(src_dir)
        .stderr_capture()
        .run()?;
    Ok(())
}
