use std::{cmp, ffi::OsStr, fmt, fs, io, path::Path};

use crate::{utils::bytes::starts_with, Scene};

type Reader<B> = fn(&Path) -> io::Result<B>;

pub struct Loader<B = Vec<u8>> {
    reader: Reader<B>,
    merge_meshes: bool,
    // STL config
    #[cfg(feature = "stl")]
    stl_parse_color: bool,
}

fn default_reader(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

impl Default for Loader<Vec<u8>> {
    fn default() -> Self {
        Self {
            reader: default_reader,
            merge_meshes: false,
            #[cfg(feature = "stl")]
            stl_parse_color: false,
        }
    }
}

impl<B: AsRef<[u8]>> Loader<B> {
    /// Sets whether or not to merge meshes at load time.
    ///
    /// If set to `true`, it is guaranteed that there is exactly one mesh in the
    /// loaded `Scene` (i.e., `scene.meshes.len() == 1`).
    ///
    /// Default: `false`
    #[must_use]
    pub fn merge_meshes(mut self, enable: bool) -> Self {
        self.merge_meshes = enable;
        self
    }

    /// Use the given function as a file reader of this loader.
    ///
    /// Default: [`std::fs::read`]
    ///
    /// # Example
    ///
    /// This is useful if you want to load a mesh from a location that the
    /// default reader does not support.
    ///
    /// ```
    /// use std::fs;
    ///
    /// use mesh_loader::Loader;
    ///
    /// let loader = Loader::default().custom_reader(|path| {
    ///     match path.to_str() {
    ///         Some(url) if url.starts_with("https://") || url.starts_with("http://") => {
    ///             // Fetch online file
    ///             // ...
    /// #           unimplemented!()
    ///         }
    ///         _ => fs::read(path), // Otherwise, read from a file (same as the default reader)
    ///     }
    /// });
    /// ```
    #[must_use]
    pub fn custom_reader(mut self, reader: Reader<B>) -> Self {
        self.reader = reader;
        self
    }

    /// Creates a new loader with the given file reader.
    ///
    /// This is similar to [`Loader::default().custom_reader()`](Self::custom_reader),
    /// but the reader can return a non-`Vec<u8>` type.
    ///
    /// # Example
    ///
    /// This is useful when using mmap.
    ///
    /// ```
    /// use std::fs::File;
    ///
    /// use memmap2::Mmap;
    /// use mesh_loader::Loader;
    ///
    /// let loader = Loader::with_custom_reader(|path| unsafe { Mmap::map(&File::open(path)?) });
    /// ```
    #[must_use]
    pub fn with_custom_reader(reader: Reader<B>) -> Self {
        Self {
            reader,
            merge_meshes: false,
            #[cfg(feature = "stl")]
            stl_parse_color: false,
        }
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) -> io::Result<Scene> {
        self.load_with_reader(path.as_ref(), self.reader)
    }
    pub fn load_with_reader<P: AsRef<Path>, F: FnMut(&Path) -> io::Result<B>>(
        &self,
        path: P,
        mut reader: F,
    ) -> io::Result<Scene> {
        let path = path.as_ref();
        self.load_from_slice_with_reader(reader(path)?.as_ref(), path, reader)
    }
    pub fn load_from_slice<P: AsRef<Path>>(&self, bytes: &[u8], path: P) -> io::Result<Scene> {
        self.load_from_slice_with_reader(bytes, path.as_ref(), self.reader)
    }
    pub fn load_from_slice_with_reader<P: AsRef<Path>, F: FnMut(&Path) -> io::Result<B>>(
        &self,
        bytes: &[u8],
        path: P,
        #[allow(unused_variables)] reader: F,
    ) -> io::Result<Scene> {
        let path = path.as_ref();
        match detect_file_type(path, bytes) {
            #[cfg(feature = "stl")]
            FileType::Stl => self.load_stl_from_slice(bytes, path),
            #[cfg(not(feature = "stl"))]
            FileType::Stl => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "'stl' feature of mesh-loader must be enabled to parse STL file ({path:?})",
            )),
            #[cfg(feature = "collada")]
            FileType::Collada => self.load_collada_from_slice(bytes, path),
            #[cfg(not(feature = "collada"))]
            FileType::Collada => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "'collada' feature of mesh-loader must be enabled to parse COLLADA file ({path:?})",
            )),
            #[cfg(feature = "obj")]
            FileType::Obj => self.load_obj_from_slice_with_reader(bytes, path, reader),
            #[cfg(not(feature = "obj"))]
            FileType::Obj => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "'obj' feature of mesh-loader must be enabled to parse OBJ file ({path:?})",
            )),
            FileType::Unknown => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported or unrecognized file type {path:?}",
            )),
        }
    }

    #[cfg(feature = "stl")]
    pub fn load_stl<P: AsRef<Path>>(&self, path: P) -> io::Result<Scene> {
        let path = path.as_ref();
        self.load_stl_from_slice((self.reader)(path)?.as_ref(), path)
    }
    #[cfg(feature = "stl")]
    pub fn load_stl_from_slice<P: AsRef<Path>>(&self, bytes: &[u8], path: P) -> io::Result<Scene> {
        let scene =
            crate::stl::from_slice_internal(bytes, Some(path.as_ref()), self.stl_parse_color)?;
        Ok(self.post_process(scene))
    }
    #[cfg(feature = "stl")]
    #[must_use]
    pub fn stl_parse_color(mut self, enable: bool) -> Self {
        self.stl_parse_color = enable;
        self
    }

    #[cfg(feature = "collada")]
    pub fn load_collada<P: AsRef<Path>>(&self, path: P) -> io::Result<Scene> {
        let path = path.as_ref();
        self.load_collada_from_slice((self.reader)(path)?.as_ref(), path)
    }
    #[cfg(feature = "collada")]
    pub fn load_collada_from_slice<P: AsRef<Path>>(
        &self,
        bytes: &[u8],
        path: P,
    ) -> io::Result<Scene> {
        let scene = crate::collada::from_slice_internal(bytes, Some(path.as_ref()))?;
        Ok(self.post_process(scene))
    }

    #[cfg(feature = "obj")]
    pub fn load_obj<P: AsRef<Path>>(&self, path: P) -> io::Result<Scene> {
        self.load_obj_with_reader(path.as_ref(), self.reader)
    }
    #[cfg(feature = "obj")]
    pub fn load_obj_from_slice<P: AsRef<Path>>(&self, bytes: &[u8], path: P) -> io::Result<Scene> {
        self.load_obj_from_slice_with_reader(bytes, path.as_ref(), self.reader)
    }
    #[cfg(feature = "obj")]
    pub fn load_obj_with_reader<P: AsRef<Path>, F: FnMut(&Path) -> io::Result<B>>(
        &self,
        path: P,
        mut reader: F,
    ) -> io::Result<Scene> {
        let path = path.as_ref();
        self.load_obj_from_slice_with_reader(reader(path)?.as_ref(), path, reader)
    }
    #[cfg(feature = "obj")]
    pub fn load_obj_from_slice_with_reader<P: AsRef<Path>, F: FnMut(&Path) -> io::Result<B>>(
        &self,
        bytes: &[u8],
        path: P,
        reader: F,
    ) -> io::Result<Scene> {
        let scene = crate::obj::from_slice(bytes, Some(path.as_ref()), reader)?;
        Ok(self.post_process(scene))
    }

    #[cfg(any(feature = "collada", feature = "obj", feature = "stl"))]
    fn post_process(&self, mut scene: Scene) -> Scene {
        if self.merge_meshes && scene.meshes.len() != 1 {
            scene.meshes = vec![crate::Mesh::merge(scene.meshes)];
            // TODO
            scene.materials = vec![crate::Material::default()];
        }
        scene
    }
}

impl fmt::Debug for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Loader");
        d.field("merge_meshes", &self.merge_meshes);
        #[cfg(feature = "stl")]
        d.field("stl_parse_color", &self.stl_parse_color);
        d.finish_non_exhaustive()
    }
}

enum FileType {
    Stl,
    Collada,
    Obj,
    Unknown,
}

fn detect_file_type(path: &Path, bytes: &[u8]) -> FileType {
    match path.extension().and_then(OsStr::to_str) {
        Some("stl" | "STL") => return FileType::Stl,
        Some("dae" | "DAE") => return FileType::Collada,
        Some("obj" | "OBJ") => return FileType::Obj,
        _ => {}
    }
    // Fallback: If failed to detect file type from extension,
    // read the first 1024 bytes to detect the file type.
    // TODO: rewrite based on what assimp does.
    let mut s = &bytes[..cmp::min(bytes.len(), 1024)];
    while let Some((&c, s_next)) = s.split_first() {
        match c {
            b's' => {
                if starts_with(s_next, &b"solid"[1..]) {
                    return FileType::Stl;
                }
            }
            b'<' => {
                // Compare whole s instead of s_next since needle.len() == 8
                if starts_with(s, b"<COLLADA") {
                    return FileType::Collada;
                }
            }
            _ => {}
        }
        s = s_next;
    }
    FileType::Unknown
}
