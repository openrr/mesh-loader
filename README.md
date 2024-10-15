# mesh-loader

[![crates.io](https://img.shields.io/crates/v/mesh-loader?style=flat-square&logo=rust)](https://crates.io/crates/mesh-loader)
[![docs.rs](https://img.shields.io/badge/docs.rs-mesh--loader-blue?style=flat-square&logo=docs.rs)](https://docs.rs/mesh-loader)
[![msrv](https://img.shields.io/badge/msrv-1.60-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![github actions](https://img.shields.io/github/actions/workflow/status/openrr/mesh-loader/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/openrr/mesh-loader/actions)

Fast parser for 3D-model-formats.

This currently supports the following three formats commonly used in robotics:

- [STL](https://en.wikipedia.org/wiki/STL_(file_format)) (.stl)
- [COLLADA](https://en.wikipedia.org/wiki/COLLADA) (.dae)
- [Wavefront OBJ](https://en.wikipedia.org/wiki/Wavefront_.obj_file) (.obj)

# Usage
`Scece` is filled with mesh data : name, vertices, normals, colors. But also its materials : opacity, shininess, texture, colors, ...

## Stl
`stl` implements `from_slice` that parses meshes from bytes of binary or ASCII STL.

__<u> Example:</u>__
```
let mut file = File::open("/your/path/to/stl/my.stl").unwrap();

let mut buffer = Vec::new();       

if let Ok(file) = file.read_to_end(&mut buffer) {
    let scene = mesh_loader::stl::from_slice(&buffer);
    if let Ok(scene) = scene {
        for mesh in &scene.meshes {
          assert_eq!(mesh.name, "*Your stl mesh name");
        }
    }
}
```
## Collada
`collada` implements `from_str` that parses meshes from a string of COLLADA text and `from_slice` that parses meshes from bytes of a COLLADA file.

__<u> Example:</u>__
```
let cube = r##"<?xml version="1.0" encoding="utf-8"?>
    <COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <asset>
        <created>2018-11-19T22:54:36</created>
        <modified>2018-11-19T22:54:36</modified>
    </asset>
    <library_geometries>
        <geometry id="Cube_001-mesh" name="Cube.001">
            <mesh>
                <source id="Cube_001-mesh-positions">
                    <float_array id="Cube_001-mesh-positions-array" count="24">-0.5 -0.5 -0.5 -0.5 -0.5 0.5 -0.5 0.5 -0.5 -0.5 0.5 0.5 0.5 -0.5 -0.5 0.5 -0.5 0.5 0.5 0.5 -0.5 0.5 0.5 0.5</float_array>
                <technique_common>
                    <accessor source="#Cube_001-mesh-positions-array" count="8" stride="3">
                    <param name="X" type="float"/>
                    <param name="Y" type="float"/>
                    <param name="Z" type="float"/>
                    </accessor>
                </technique_common>
                </source>
                    <source id="Cube_001-mesh-normals">
                        <float_array id="Cube_001-mesh-normals-array" count="18">-1 0 0 0 1 0 1 0 0 0 -1 0 0 0 -1 0 0 1</float_array>
                    <technique_common>
                        <accessor source="#Cube_001-mesh-normals-array" count="6" stride="3">
                        <param name="X" type="float"/>
                        <param name="Y" type="float"/>
                        <param name="Z" type="float"/>
                        </accessor>
                    </technique_common>
                </source>
                <vertices id="Cube_001-mesh-vertices">
                    <input semantic="POSITION" source="#Cube_001-mesh-positions"/>
                </vertices>
                <triangles material="Material-material" count="12">
                    <input semantic="VERTEX" source="#Cube_001-mesh-vertices" offset="0"/>
                    <input semantic="NORMAL" source="#Cube_001-mesh-normals" offset="1"/>
                    <p>1 0 2 0 0 0 3 1 6 1 2 1 7 2 4 2 6 2 5 3 0 3 4 3 6 4 0 4 2 4 3 5 5 5 7 5 1 0 3 0 2 0 3 1 7 1 6 1 7 2 5 2 4 2 5 3 1 3 0 3 6 4 4 4 0 4 3 5 1 5 5 5</p>
                </triangles>
            </mesh>
        </geometry>
    </library_geometries>
</COLLADA>"##;

let doc = mesh_loader::collada::from_str(&cube);

if let Ok(doc) = doc {
    for mesh in &doc.meshes {
        assert_eq!(mesh.name, "Cube_001-mesh");
    }
}

```

## Wavefront OBJ
`obj` implements `from_slice` that parses meshes from bytes of Wavefront OBJ file.

__<u> Example:</u>__
```
pub fn reader(path: &std::path::Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
  
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

let path = std::path::Path::new("/home/kgoddard/rerun_models/wheel.obj");
let mut file = File::open(&path).unwrap();
let mut buffer = Vec::new();

if let Ok(file) = file.read_to_end(&mut buffer) {
    let scene = mesh_loader::obj::from_slice(&buffer, Some(path), reader);
    if let Ok(scene) = scene {
        for mesh in &scene.meshes {
            assert_eq!(mesh.name, "*Your stl mesh name");
        }
    }
}
```