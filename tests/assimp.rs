#![allow(
    clippy::match_same_arms, // https://github.com/rust-lang/rust-clippy/issues/12044
)]

#[path = "shared/assimp.rs"]
mod assimp_helper;

use std::{collections::BTreeSet, ffi::OsStr, panic, path::Path};

use duct::cmd;
use fs_err as fs;
use walkdir::WalkDir;

#[test]
fn test() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assimp_dir = &manifest_dir.join("tests/fixtures/assimp");

    clone(assimp_dir, "assimp/assimp", &["/test/models/"]);
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
    assert_eq!(collada_models.len(), 27);
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
        if path.parent().unwrap().file_name().unwrap() == "invalid" {
            let _e = mesh_loader.load(path).unwrap_err();
            // TODO: latest assimp reject this, but old doesn't
            if matches!(filename, "box_nested_animation_4286.dae") {
                let _res = assimp_importer.read_file(path.to_str().unwrap());
            } else {
                let _e = assimp_importer
                    .read_file(path.to_str().unwrap())
                    .err()
                    .unwrap();
            }
            continue;
        }
        let (ml_scene, ml) = &load_mesh_loader(&mesh_loader, path);
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
            // More faces loaded only in CI...
            "ConcavePolygon.dae" if option_env!("CI").is_some() => continue,
            _ => {}
        }
        let (ai_scene, ai) = &load_assimp(&assimp_importer, path);

        if matches!(
            filename,
            "Cinema4D.dae"
                | "anims_with_full_rotations_between_keys.DAE"
                | "cameras.dae"
                | "earthCylindrical.DAE"
                | "kwxport_test_vcolors.dae"
                | "lights.dae"
                | "regr01.dae"
        ) {
            // TODO
            assert_ne!(ml_scene.meshes.len(), ai_scene.meshes.len());
        } else {
            assert_eq!(ml_scene.meshes.len(), ai_scene.meshes.len());
        }
        if matches!(
            filename,
            "ConcavePolygon.dae" | "cameras.dae" | "lights.dae" | "teapot_instancenodes.DAE"
        ) {
            // TODO
            assert_ne!(ml.faces.len(), ai.faces.len());
        } else {
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
                    | "kwxport_test_vcolors.dae"
                    | "regr01.dae"
                    | "sphere.dae"
                    | "teapots.DAE"
            ) {
                assert_faces(ml, ai);
            }
        }
        // TODO
        if !matches!(
            filename,
            "anims_with_full_rotations_between_keys.DAE"
                | "ConcavePolygon.dae"
                | "duck.dae"
                | "lights.dae"
                | "teapot_instancenodes.DAE"
        ) {
            // TODO
            // assert_eq!(ml.vertices.len(), ai.vertices.len());
            if matches!(
                filename,
                "box_nested_animation.dae"
                    | "cameras.dae"
                    | "Cinema4D.dae"
                    | "cube_tristrips.dae"
                    | "earthCylindrical.DAE"
                    | "kwxport_test_vcolors.dae"
                    | "regr01.dae"
                    | "teapots.DAE"
            ) {
                // TODO
                panic::catch_unwind(|| assert_vertices(ml, ai, f32::EPSILON * 1000.)).unwrap_err();
            } else {
                assert_vertices(ml, ai, f32::EPSILON * 1000.);
            }
            if matches!(
                filename,
                "AsXML.xml"
                    | "cameras.dae"
                    | "COLLADA.dae"
                    | "cube_UTF16LE.dae"
                    | "cube_UTF8BOM.dae"
                    | "cube_emptyTags.dae"
                    | "cube_xmlspecialchars.dae"
                    | "sphere.dae"
            ) {
                // TODO
                assert_ne!(ml.normals.len(), ai.normals.len());
            } else {
                assert_eq!(ml.normals.len(), ai.normals.len());
                if matches!(
                    filename,
                    "Cinema4D.dae"
                        | "cube_tristrips.dae"
                        | "earthCylindrical.DAE"
                        | "kwxport_test_vcolors.dae"
                        | "regr01.dae"
                        | "AsXML.xml"
                ) {
                    panic::catch_unwind(|| {
                        assert_full_matches(&ml.normals, &ai.normals, f32::EPSILON * 1000.);
                    })
                    .unwrap_err();
                } else {
                    assert_full_matches(&ml.normals, &ai.normals, f32::EPSILON * 100.);
                }
            }
            if matches!(
                filename,
                "Cinema4D.dae"
                    | "earthCylindrical.DAE"
                    | "regr01.dae"
                    | "sphere.dae"
                    | "kwxport_test_vcolors.dae"
            ) {
                panic::catch_unwind(|| {
                    assert_full_matches(&ml.texcoords[0], &ai.texcoords[0], f32::EPSILON * 1000.);
                })
                .unwrap_err();
            } else {
                assert_full_matches(&ml.texcoords[0], &ai.texcoords[0], f32::EPSILON);
            }
            if matches!(filename, "cube_with_2UVs.DAE") {
                panic::catch_unwind(|| {
                    assert_full_matches(&ml.texcoords[1], &ai.texcoords[1], f32::EPSILON * 1000.);
                })
                .unwrap_err();
            } else {
                assert_full_matches(&ml.texcoords[1], &ai.texcoords[1], f32::EPSILON);
            }
            if matches!(filename, "kwxport_test_vcolors.dae") {
                panic::catch_unwind(|| {
                    assert_full_matches(&ml.colors[0], &ai.colors[0], f32::EPSILON * 1000.);
                })
                .unwrap_err();
            } else {
                assert_full_matches(&ml.colors[0], &ai.colors[0], f32::EPSILON);
            }
            assert_full_matches(&ml.colors[1], &ai.colors[1], f32::EPSILON);
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
        let (ml_scene, ml) = &load_mesh_loader(&mesh_loader, path);
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
            // Less or more faces loaded only in CI...
            "box_without_lineending.obj"
            | "concave_polygon.obj"
            | "cube_with_vertexcolors_uni.obj"
            | "cube_with_vertexcolors.obj"
            | "regr_3429812.obj"
            | "space_in_material_name.obj"
                if option_env!("CI").is_some() =>
            {
                continue
            }
            _ => {}
        }
        let (ai_scene, ai) = &load_assimp(&assimp_importer, path);

        if matches!(
            filename,
            "box_UTF16BE.obj"
                | "cube_usemtl.obj"
                | "regr01.obj"
                | "testpoints.obj"
                | "regr_3429812.obj"
                | "spider.obj"
                | "testline.obj"
        ) {
            // TODO
            assert_ne!(ml_scene.meshes.len(), ai_scene.meshes.len());
        } else {
            assert_eq!(ml_scene.meshes.len(), ai_scene.meshes.len());
        }
        if matches!(
            filename,
            "box_UTF16BE.obj"
                | "box_longline.obj"
                | "concave_polygon.obj"
                | "space_in_material_name.obj"
        ) {
            // TODO
            assert_ne!(ml.faces.len(), ai.faces.len());
        } else {
            assert_eq!(ml.faces.len(), ai.faces.len());
            if matches!(
                filename,
                "box_mat_with_spaces.obj"
                    | "box_without_lineending.obj"
                    | "box.obj"
                    | "malformed2.obj"
                    | "regr_3429812.obj"
                    | "cube_usemtl.obj"
                    | "testmixed.obj"
                    | "regr01.obj"
                    | "spider.obj"
            ) {
                panic::catch_unwind(|| assert_faces(ml, ai)).unwrap_err();
            } else {
                assert_faces(ml, ai);
            }
            if matches!(filename, "cube_usemtl.obj" | "regr01.obj" | "spider.obj") {
                panic::catch_unwind(|| assert_vertices(ml, ai, f32::EPSILON * 1000.)).unwrap_err();
            } else {
                assert_vertices(ml, ai, f32::EPSILON * 10.);
            }
            if matches!(filename, "cube_usemtl.obj" | "spider.obj") {
                panic::catch_unwind(|| {
                    assert_full_matches(&ml.normals, &ai.normals, f32::EPSILON * 1000.);
                })
                .unwrap_err();
            } else {
                assert_full_matches(&ml.normals, &ai.normals, f32::EPSILON);
            }
            assert_full_matches(&ml.texcoords[0], &ai.texcoords[0], f32::EPSILON);
            assert_full_matches(&ml.texcoords[1], &ai.texcoords[1], f32::EPSILON);
            if matches!(filename, "only_a_part_of_vertexcolors.obj") {
                panic::catch_unwind(|| {
                    assert_full_matches(&ml.colors[0], &ai.colors[0], f32::EPSILON * 1000.);
                })
                .unwrap_err();
            } else {
                assert_full_matches(&ml.colors[0], &ai.colors[0], f32::EPSILON);
            }
            assert_full_matches(&ml.colors[1], &ai.colors[1], f32::EPSILON);
        }
    }

    // STL
    for path in &stl_models {
        eprintln!();
        eprintln!("parsing {:?}", path.strip_prefix(manifest_dir).unwrap());
        let filename = path.file_name().unwrap().to_str().unwrap();

        // mesh-loader
        let (ml_scene, ml) = &load_mesh_loader(&mesh_loader, path);
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
        let (ai_scene, ai) = &load_assimp(&assimp_importer, path);

        if matches!(
            filename,
            "triangle_with_empty_solid.stl" | "triangle_with_two_solids.stl"
        ) {
            // TODO
            assert_ne!(ml_scene.meshes.len(), ai_scene.meshes.len());
        } else {
            assert_eq!(ml_scene.meshes.len(), ai_scene.meshes.len());
        }
        assert_faces(ml, ai);
        assert_full_matches(&ml.vertices, &ai.vertices, f32::EPSILON * 10.);
        assert_full_matches(&ml.texcoords[0], &ai.texcoords[0], f32::EPSILON);
        assert_full_matches(&ml.texcoords[1], &ai.texcoords[1], f32::EPSILON);
        assert_full_matches(&ml.normals, &ai.normals, f32::EPSILON);
        assert_full_matches(&ml.colors[0], &ai.colors[0], f32::EPSILON);
        assert_full_matches(&ml.colors[1], &ai.colors[1], f32::EPSILON);
    }
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
    assert_eq!(ml.faces.len(), ai.faces.len());
    // assert_eq!(ml.vertices.len(), ai.vertices.len());
    for (i, (ml_face, ai_face)) in ml.faces.iter().zip(&ai.faces).enumerate() {
        for j in 0..ml_face.len() {
            let (ml, ai) = (
                ml.vertices[ml_face[j] as usize],
                ai.vertices[ai_face[j] as usize],
            );
            for k in 0..ml.len() {
                let (a, b) = (ml[k], ai[k]);
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
    // for (i, (ml, ai)) in ml.vertices.iter().zip(&ai.vertices).enumerate() {
    //     for j in 0..ml.len() {
    //         let (a, b) = (ml[j], ai[j]);
    //         assert!(
    //             (a - b).abs() < eps,
    //             "assertion failed: `(left !== right)` \
    //                         (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
    //                         real diff: `{:?}`) at vertices[{i}][{j}]",
    //             (a - b).abs()
    //         );
    //     }
    // }
}
// Asserts length, order, and values are all matched.
#[track_caller]
fn assert_full_matches<const N: usize>(a: &[[f32; N]], b: &[[f32; N]], eps: f32) {
    assert_eq!(a.len(), b.len());
    for (i, (a, b)) in a.iter().zip(b).enumerate() {
        for j in 0..a.len() {
            let (a, b) = (a[j], b[j]);
            assert!(
                (a - b).abs() < eps,
                "assertion failed: `(left !== right)` \
                            (left: `{a:?}`, right: `{b:?}`, expect diff: `{eps:?}`, \
                            real diff: `{:?}`) at [{i}][{j}]",
                (a - b).abs()
            );
        }
    }
}

#[track_caller]
fn load_mesh_loader(
    loader: &mesh_loader::Loader,
    path: &Path,
) -> (mesh_loader::Scene, mesh_loader::Mesh) {
    let scene = loader.load(path).unwrap();
    for (i, m) in scene.meshes.iter().enumerate() {
        eprintln!("ml.meshes[{i}]={m:?}");
    }
    let merged_mesh = mesh_loader::Mesh::merge(scene.meshes.clone());
    eprintln!("merge(ml.meshes)={merged_mesh:?}");
    for (i, colors) in merged_mesh.colors.iter().enumerate() {
        for (j, c) in colors.iter().enumerate() {
            for (k, &v) in c.iter().enumerate() {
                assert!(
                    v >= 0. && v <= 100.,
                    "colors[{i}][{j}][{k}] should be clamped in 0..=100, but is {v}"
                );
            }
        }
    }
    (scene, merged_mesh)
}
#[track_caller]
fn load_assimp(
    importer: &assimp::Importer,
    path: &Path,
) -> (mesh_loader::Scene, mesh_loader::Mesh) {
    let ai_scene = importer.read_file(path.to_str().unwrap()).unwrap();
    let scene = assimp_helper::assimp_scene_to_scene(&ai_scene);
    for (i, m) in scene.meshes.iter().enumerate() {
        eprintln!("ai.meshes[{i}]={m:?}");
    }
    let merged_mesh = mesh_loader::Mesh::merge(scene.meshes.clone());
    eprintln!("merge(ai.meshes)={merged_mesh:?}");
    (scene, merged_mesh)
}

#[track_caller]
fn clone(src_dir: &Path, repository: &str, sparse_checkout: &[&str]) {
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
        fs::create_dir_all(src_dir.parent().unwrap()).unwrap();
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
        .run()
        .unwrap();
    }
    cmd!("git", "sparse-checkout", "init")
        .dir(src_dir)
        .run()
        .unwrap();
    let mut out = String::from("/*\n!/*/\n"); // always download top-level files
    out.push_str(&sparse_checkout.join("\n"));
    fs::write(src_dir.join(".git/info/sparse-checkout"), out).unwrap();
    cmd!("git", "checkout")
        .dir(src_dir)
        .stdout_capture()
        .run()
        .unwrap();
    cmd!("git", "clean", "-df")
        .dir(src_dir)
        .stdout_capture()
        .run()
        .unwrap();
    // TODO: use stash?
    cmd!("git", "checkout", ".")
        .dir(src_dir)
        .stderr_capture()
        .run()
        .unwrap();
}
