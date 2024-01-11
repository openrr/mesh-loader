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
    let mut download_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    download_dir.push("tests/fixtures");

    clone(&download_dir, "assimp/assimp", &["/test/models/"]).unwrap();
    let models = &download_dir.join("assimp/assimp/test/models");

    let mut collada_models = BTreeSet::new();
    // let mut obj_models = BTreeSet::new();
    let mut stl_models = BTreeSet::new();
    for e in WalkDir::new(models).into_iter().filter_map(Result::ok) {
        let path = e.path();
        match path.extension().and_then(OsStr::to_str) {
            Some("dae" | "DAE") => collada_models.insert(path.to_owned()),
            // Some("obj" | "OBJ") => obj_models.insert(path.to_owned()),
            Some("stl" | "STL") => stl_models.insert(path.to_owned()),
            _ => false,
        };
    }
    assert_eq!(collada_models.len(), 25);
    // assert_eq!(obj_models.len(), 26);
    assert_eq!(stl_models.len(), 8);

    let mut assimp_importer = assimp::Importer::new();
    assimp_importer.pre_transform_vertices(|x| x.enable = true);
    assimp_importer.collada_ignore_up_direction(true);
    assimp_importer.triangulate(true);

    // COLLADA
    for (i, path) in collada_models.iter().enumerate() {
        let filename = path.file_name().unwrap().to_str().unwrap();
        eprintln!("parsing {path:?} (i={i})");
        if matches!(filename, "cube_UTF16LE.dae") {
            // not utf8
            continue;
        }

        // mesh-loader
        let ml = mesh_loader::collada::from_slice(&fs::read(path).unwrap()).unwrap();
        for (i, m) in ml.meshes.iter().enumerate() {
            eprintln!("ml.meshes[{i}].name = {:?}", m.name);
        }
        let ml = mesh_loader::Mesh::merge(ml.meshes);

        match filename {
            // assimp parse error: Cannot parse string \"  0.0 0.0 0.0 1.0  \" as a real number: does not start with digit or decimal point followed by digit.
            "library_animation_clips.dae" => continue,
            // assimp error: "Collada: File came out empty. Something is wrong here."
            "cube_tristrips.dae" if option_env!("CI").is_some() => continue,
            _ => {}
        }

        // assimp
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
        if !matches!(i, 3 | 6 | 19 | 23) {
            assert_eq!(ml.faces.len(), ai_faces.len());
            // TODO
            if !matches!(i, 0 | 2 | 4 | 8 | 9 | 13 | 14 | 21 | 24) {
                for (ml, ai) in ml.faces.iter().copied().zip(ai_faces) {
                    assert_eq!(ml, ai);
                }
            }
        }
        // TODO
        if !matches!(i, 0 | 3 | 4 | 6 | 8 | 9 | 13 | 14 | 19 | 21 | 23) {
            assert_eq!(ml.vertices.len(), ai_vertices.len());
            // TODO
            if !matches!(i, 2 | 5 | 11 | 12 | 16 | 17 | 20 | 24) {
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

    // STL
    for (i, path) in stl_models.iter().enumerate() {
        let filename = path.file_name().unwrap().to_str().unwrap();
        eprintln!("parsing {path:?} (i={i})");

        // mesh-loader
        let ml = mesh_loader::stl::from_slice(&fs::read(path).unwrap()).unwrap();
        for (i, m) in ml.meshes.iter().enumerate() {
            eprintln!("ml.meshes[{i}].name = {:?}", m.name);
        }
        let ml = mesh_loader::Mesh::merge(ml.meshes);

        match filename {
            // assimp error: "STL: ASCII file is empty or invalid; no data loaded"
            "triangle_with_empty_solid.stl" if option_env!("CI").is_some() => continue,
            _ => {}
        }

        // assimp
        let ai = assimp_importer.read_file(path.to_str().unwrap()).unwrap();
        assert_eq!(ai.num_meshes, 1);
        let ai = ai.mesh(0).unwrap();

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
        for (j, (ml, ai)) in ml
            .vertices
            .iter()
            .copied()
            .zip(ai.vertex_iter().map(|f| [f.x, f.y, f.z]))
            .enumerate()
        {
            for i in 0..ml.len() {
                let eps = f32::EPSILON * 10.;
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
