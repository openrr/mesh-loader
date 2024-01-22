#![allow(
    clippy::match_same_arms, // https://github.com/rust-lang/rust-clippy/issues/12044
)]

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    str,
};

use anyhow::Result;
use duct::cmd;
use fs_err as fs;
use walkdir::WalkDir;

#[test]
fn test() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let download_dir = &manifest_dir.join("tests/fixtures");

    clone(download_dir, "assimp/assimp", &["/test/models/"]).unwrap();
    let models = &download_dir.join("assimp/assimp/test/models");

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
        let ml = mesh_loader::Mesh::merge(ml.meshes);
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
        let ai_vertices = ai
            .mesh_iter()
            .flat_map(|mesh| {
                mesh.vertex_iter()
                    .map(|v| [v.x, v.y, v.z])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut last = 0;
        let ai_faces = ai
            .mesh_iter()
            .flat_map(|mesh| {
                let f = mesh
                    .face_iter()
                    .filter_map(|f| {
                        if f.num_indices == 3 {
                            Some([f[0] + last, f[1] + last, f[2] + last])
                        } else {
                            assert!(f.num_indices < 3, "should be triangulated");
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                if !f.is_empty() {
                    last = f.last().unwrap()[2] + 1;
                }
                f
            })
            .collect::<Vec<_>>();

        // TODO
        if !matches!(
            filename,
            "ConcavePolygon.dae" | "cameras.dae" | "lights.dae" | "teapot_instancenodes.DAE"
        ) {
            assert_eq!(ml.faces.len(), ai_faces.len());
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
                for (ml, ai) in ml.faces.iter().copied().zip(ai_faces) {
                    assert_eq!(ml, ai);
                }
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
            assert_eq!(ml.vertices.len(), ai_vertices.len());
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
                let mut first = true;
                let mut x = 1.;
                for (j, (ml, ai)) in ml.vertices.iter().copied().zip(ai_vertices).enumerate() {
                    for i in 0..ml.len() {
                        let eps = f32::EPSILON * 1000.;
                        let (a, b) = (ml[i], ai[i]);
                        if first {
                            first = false;
                            if (a - b).abs() < eps {
                                continue;
                            }
                            // TODO
                            if (a - b * 100.).abs() < eps {
                                x = 100.;
                                continue;
                            }
                        }
                        assert!(
                            (a - b * x).abs() < eps,
                            "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, real diff: `{:?}`) \
                            at vertices[{j}][{i}]",
                            (a - b).abs()
                        );
                    }
                }
            }
        }
    }

    // OBJ
    for path in &obj_models {
        eprintln!();
        eprintln!("parsing {:?}", path.strip_prefix(manifest_dir).unwrap());
        let filename = path.file_name().unwrap().to_str().unwrap();
        match filename {
            // no mesh
            "point_cloud.obj"
            // no face
            | "testline.obj" | "testpoints.obj"
             => continue,
            _ => {}
        }

        // mesh-loader
        match filename {
            // number parsing issue
            "number_formats.obj"
            // TODO: should not be allowed
            | "empty.obj" | "malformed2.obj" => continue,
            _ => {}
        }
        if path.parent().unwrap().file_name().unwrap() == "invalid" {
            let _e = mesh_loader.load(path).unwrap_err();
            let _e = assimp_importer
                .read_file(path.to_str().unwrap())
                .map(drop)
                .unwrap_err();
            continue;
        }
        let ml = mesh_loader.load(path).unwrap();
        for (i, m) in ml.meshes.iter().enumerate() {
            eprintln!("ml.meshes[{i}]={m:?}");
        }
        let ml = mesh_loader::Mesh::merge(ml.meshes);
        eprintln!("merge(ml.meshes)={ml:?}");
        assert_ne!(ml.vertices.len(), 0);
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
            // segmentation fault...
            "box.obj"
            | "box_longline.obj"
            | "box_mat_with_spaces.obj"
            | "box_without_lineending.obj"
            | "multiple_spaces.obj"
            | "only_a_part_of_vertexcolors.obj"
            | "regr_3429812.obj"
            | "regr01.obj"
            | "testmixed.obj" => continue,
            // no mesh...
            "box_UTF16BE.obj" => continue,
            // less number of faces loaded...
            "cube_with_vertexcolors.obj" | "cube_with_vertexcolors_uni.obj"
                if option_env!("CI").is_some() =>
            {
                continue
            }
            _ => {}
        }
        let ai = assimp_importer.read_file(path.to_str().unwrap()).unwrap();
        // assert_eq!(ai.num_meshes, 1);
        // assert_eq!(ai.num_meshes, ai.num_materials);
        let ai = ai.mesh(0).unwrap();
        // assert_eq!(ai.num_vertices, ai.num_faces * 3);
        assert_eq!(ai.num_vertices as usize, ai.vertex_iter().count());
        assert_eq!(ai.num_vertices as usize, ai.normal_iter().count());
        if ai.has_texture_coords(0) {
            assert_eq!(ai.num_vertices as usize, ai.texture_coords_iter(0).count());
        }
        if ai.has_vertex_colors(0) {
            assert_eq!(ai.num_vertices as usize, ai.vertex_color_iter(0).count());
        }
        assert!(!ai.has_texture_coords(1));

        // TODO
        if !matches!(
            filename,
            "concave_polygon.obj" | "space_in_material_name.obj" | "spider.obj" | "cube_usemtl.obj"
        ) {
            assert_eq!(ml.faces.len(), ai.num_faces as usize);
            for (ml, ai) in ml
                .faces
                .iter()
                .copied()
                .zip(ai.face_iter().map(|f| [f[0], f[1], f[2]]))
            {
                assert_eq!(ml, ai);
            }
        }
        if !matches!(
            filename,
            "concave_polygon.obj" | "space_in_material_name.obj" | "spider.obj" | "cube_usemtl.obj"
        ) {
            assert_eq!(ml.vertices.len(), ai.num_vertices as usize);
            assert_eq!(ml.normals.len(), ai.num_vertices as usize);
            if !matches!(filename, "cube_usemtl.obj") {
                for (j, (ml, ai)) in ml
                    .vertices
                    .iter()
                    .copied()
                    .zip(ai.vertex_iter().map(|f| [f.x, f.y, f.z]))
                    .enumerate()
                {
                    let eps = f32::EPSILON * 10.;
                    for i in 0..ml.len() {
                        let (a, b) = (ml[i], ai[i]);
                        assert!(
                            (a - b).abs() < eps,
                            "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at vertices[{j}][{i}]",
                            (a - b).abs()
                        );
                    }
                }
                for (j, (ml, ai)) in ml
                    .normals
                    .iter()
                    .copied()
                    .zip(ai.normal_iter().map(|f| [f.x, f.y, f.z]))
                    .enumerate()
                {
                    let eps = f32::EPSILON;
                    for i in 0..ml.len() {
                        let (a, b) = (ml[i], ai[i]);
                        assert!(
                            (a - b).abs() < eps,
                            "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at normals[{j}][{i}]",
                            (a - b).abs()
                        );
                    }
                }
            }
            if ai.has_vertex_colors(0) {
                assert_eq!(ml.colors[0].len(), ai.num_vertices as usize);
                for (j, (ml, ai)) in ml.colors[0]
                    .iter()
                    .copied()
                    .zip(ai.vertex_color_iter(0).map(|f| [f.r, f.g, f.b, f.a]))
                    .enumerate()
                {
                    let eps = f32::EPSILON;
                    for i in 0..ml.len() {
                        let (a, b) = (ml[i], ai[i]);
                        assert!(
                            (a - b).abs() < eps,
                            "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at colors[0][{j}][{i}]",
                            (a - b).abs()
                        );
                        assert!(a >= 0. && a <= 100.);
                    }
                }
            } else {
                assert_eq!(ml.colors[0].len(), 0);
            }
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
        let ml = mesh_loader::Mesh::merge(ml.meshes);
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
        let ai = ai.mesh(0).unwrap();
        assert_eq!(ai.num_vertices, ai.num_faces * 3);
        assert_eq!(ai.num_vertices as usize, ai.vertex_iter().count());
        assert_eq!(ai.num_vertices as usize, ai.normal_iter().count());
        assert!(!ai.has_texture_coords(0));
        if ai.has_vertex_colors(0) {
            assert_eq!(ai.num_vertices as usize, ai.vertex_color_iter(0).count());
        }
        assert!(!ai.has_texture_coords(1));

        assert_eq!(ml.faces.len(), ai.num_faces as usize);
        for (ml, ai) in ml
            .faces
            .iter()
            .copied()
            .zip(ai.face_iter().map(|f| [f[0], f[1], f[2]]))
        {
            assert_eq!(ml, ai);
        }
        assert_eq!(ml.vertices.len(), ai.num_vertices as usize);
        assert_eq!(ml.normals.len(), ai.num_vertices as usize);
        for (j, (ml, ai)) in ml
            .vertices
            .iter()
            .copied()
            .zip(ai.vertex_iter().map(|f| [f.x, f.y, f.z]))
            .enumerate()
        {
            let eps = f32::EPSILON * 10.;
            for i in 0..ml.len() {
                let (a, b) = (ml[i], ai[i]);
                assert!(
                    (a - b).abs() < eps,
                    "assertion failed: `(left !== right)` \
                    (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, real diff: `{:?}`) \
                    at vertices[{j}][{i}]",
                    (a - b).abs()
                );
            }
        }
        for (j, (ml, ai)) in ml
            .normals
            .iter()
            .copied()
            .zip(ai.normal_iter().map(|f| [f.x, f.y, f.z]))
            .enumerate()
        {
            let eps = f32::EPSILON;
            for i in 0..ml.len() {
                let (a, b) = (ml[i], ai[i]);
                assert!(
                    (a - b).abs() < eps,
                    "assertion failed: `(left !== right)` \
                    (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, real diff: `{:?}`) \
                    at normals[{j}][{i}]",
                    (a - b).abs()
                );
            }
        }
        if ai.has_vertex_colors(0) {
            assert_eq!(ml.colors[0].len(), ai.num_vertices as usize);
            for (j, (ml, ai)) in ml.colors[0]
                .iter()
                .copied()
                .zip(ai.vertex_color_iter(0).map(|f| [f.r, f.g, f.b, f.a]))
                .enumerate()
            {
                let eps = f32::EPSILON;
                for i in 0..ml.len() {
                    let (a, b) = (ml[i], ai[i]);
                    assert!(
                        (a - b).abs() < eps,
                        "assertion failed: `(left !== right)` \
                        (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                        real diff: `{:?}`) at colors[0][{j}][{i}]",
                        (a - b).abs()
                    );
                    assert!(a >= 0. && a <= 100.);
                }
            }
        } else {
            assert_eq!(ml.colors[0].len(), 0);
        }
    }
}

fn clone(download_dir: &Path, repository: &str, sparse_checkout: &[&str]) -> Result<PathBuf> {
    let name = repository.strip_suffix(".git").unwrap_or(repository);
    assert!(!name.contains("://"), "{}", name);
    let repository = if repository.contains("://") {
        repository.to_owned()
    } else {
        format!("https://github.com/{repository}.git")
    };
    let src_dir = download_dir.join(name);
    if !src_dir.exists() {
        fs::create_dir_all(src_dir.parent().unwrap())?;
        if sparse_checkout.is_empty() {
            cmd!("git", "clone", "--depth", "1", repository, &src_dir).run()?;
        } else {
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
    }
    if !sparse_checkout.is_empty() {
        cmd!("git", "sparse-checkout", "init").dir(&src_dir).run()?;
        let mut out = String::from("/*\n!/*/\n"); // always download top-level files
        out.push_str(&sparse_checkout.join("\n"));
        fs::write(src_dir.join(".git/info/sparse-checkout"), out)?;
        cmd!("git", "checkout")
            .dir(&src_dir)
            .stdout_capture()
            .run()?;
    }
    cmd!("git", "clean", "-df")
        .dir(&src_dir)
        .stdout_capture()
        .run()?;
    // TODO: use stash?
    cmd!("git", "checkout", ".")
        .dir(&src_dir)
        .stderr_capture()
        .run()?;
    Ok(src_dir)
}
