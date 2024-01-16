use std::{ffi::OsStr, fmt, fs, io, path::Path};

use crate::Scene;

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
    /// use mesh_loader::Loader;
    /// use std::fs;
    ///
    /// let loader = Loader::default().custom_reader(|path| {
    ///     match path.to_str() {
    ///         Some(url) if url.starts_with("http://") || url.starts_with("https://") => {
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
    /// use memmap2::Mmap;
    /// use mesh_loader::Loader;
    /// use std::fs::File;
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
        self.load_(path.as_ref())
    }
    fn load_(&self, path: &Path) -> io::Result<Scene> {
        self.load_from_slice_((self.reader)(path)?.as_ref(), path.as_ref())
    }
    pub fn load_from_slice<P: AsRef<Path>>(&self, bytes: &[u8], path: P) -> io::Result<Scene> {
        self.load_from_slice_(bytes, path.as_ref())
    }
    fn load_from_slice_(
        &self,
        #[allow(unused_variables)] bytes: &[u8],
        path: &Path,
    ) -> io::Result<Scene> {
        match path.extension().and_then(OsStr::to_str) {
            #[cfg(feature = "stl")]
            Some("stl" | "STL") => self.load_stl_from_slice_(bytes, path),
            #[cfg(not(feature = "stl"))]
            Some("stl" | "STL") => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "'stl' feature of mesh-loader must be enabled to parse STL file ({path:?})",
            )),
            #[cfg(feature = "collada")]
            Some("dae" | "DAE") => self.load_from_slice_(bytes, path),
            #[cfg(not(feature = "collada"))]
            Some("dae" | "DAE") => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "'collada' feature of mesh-loader must be enabled to parse COLLADA file ({path:?})",
            )),
            // #[cfg(feature = "obj")]
            // Some("obj" | "OBJ") => self.load_obj_(path),
            // #[cfg(not(feature = "obj"))]
            // Some("obj" | "OBJ") => Err(io::Error::new(
            //     io::ErrorKind::Unsupported,
            //     "'obj' feature of mesh-loader must be enabled to parse OBJ file ({path:?})",
            // )),
            _ => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported or unrecognized file type {path:?}",
            )),
        }
    }

    #[cfg(feature = "stl")]
    pub fn load_stl<P: AsRef<Path>>(&self, path: P) -> io::Result<Scene> {
        self.load_stl_(path.as_ref())
    }
    #[cfg(feature = "stl")]
    fn load_stl_(&self, path: &Path) -> io::Result<Scene> {
        self.load_stl_from_slice_((self.reader)(path)?.as_ref(), path)
    }
    #[cfg(feature = "stl")]
    pub fn load_stl_from_slice<P: AsRef<Path>>(&self, bytes: &[u8], path: P) -> io::Result<Scene> {
        self.load_stl_from_slice_(bytes, path.as_ref())
    }
    #[cfg(feature = "stl")]
    fn load_stl_from_slice_(&self, bytes: &[u8], path: &Path) -> io::Result<Scene> {
        let scene = crate::stl::from_slice_internal(bytes, Some(path), self.stl_parse_color)?;
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
        self.load_collada_(path.as_ref())
    }
    #[cfg(feature = "collada")]
    fn load_collada_(&self, path: &Path) -> io::Result<Scene> {
        self.load_collada_from_slice_((self.reader)(path)?.as_ref(), path)
    }
    #[cfg(feature = "collada")]
    pub fn load_collada_from_slice<P: AsRef<Path>>(
        &self,
        bytes: &[u8],
        path: P,
    ) -> io::Result<Scene> {
        self.load_collada_from_slice_(bytes, path.as_ref())
    }
    #[cfg(feature = "collada")]
    fn load_collada_from_slice_(&self, bytes: &[u8], _path: &Path) -> io::Result<Scene> {
        let scene = crate::collada::from_slice(bytes)?;
        Ok(self.post_process(scene))
    }

    #[cfg(any(feature = "collada", feature = "stl"))]
    fn post_process(&self, mut scene: Scene) -> Scene {
        if self.merge_meshes && scene.meshes.len() != 1 {
            scene.meshes = vec![crate::Mesh::merge(scene.meshes)];
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
