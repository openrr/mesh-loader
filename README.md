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
[`Scene`] is filled with mesh data : name, vertices, normals, colors, texcoords and faces. But also its materials : opacity, shininess, index of refraction, texture, colors, ...

[`Loader`] implements `load` and `load_from_slice` which will guess the file media type. But also, `load_{stl/collada/obj}` and `load_{stl/collada/obj}_from_slice` for individual formats.

### Example
```
let path = std::path::Path::new("/your/path/to/file/file.{file_format}");
let loader = mesh_loader::Loader::default();
let scene = loader.load(path);

if let Ok(scene) = scene {
    for mesh in &scene.meshes {
        assert_eq!(mesh.name, "Your mesh name");
    }
}
```