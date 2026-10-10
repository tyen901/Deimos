//! Session-owned access to retail content using Deimos's wire schemas.
//! The embedding engine owns scheduling, resource caching and rendering.
mod geometry;
mod materials;
pub use materials::{DrawMaterial, DrawSlot, MaterialSet};
mod world;
pub use geometry::{Layouts, Mesh, MeshKey};
mod scene;
pub use deimos_data;
pub use glam;
pub use scene::{MaterialBinding, RenderObjectKey, Scene, SceneCompilation};
pub use world::{World, WorldEntry};

use anyhow::{Context, Result, ensure};
use deimos_data::tag::{TagRef, WideHash};
use std::{io::Cursor, path::Path};
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{GameVersion, MarathonVersion, PackageManager};

/// The package library owns indexing, decompression and open-package reuse.
/// No global package manager is installed by this API.
pub struct Installation {
    manager: PackageManager,
}
impl Installation {
    pub fn open(root: &Path) -> Result<Self> {
        let packages = root.join("packages");
        ensure!(
            packages.is_dir(),
            "Packages directory missing: {}",
            packages.display()
        );
        let manager = PackageManager::new(
            packages,
            GameVersion::Marathon(MarathonVersion::Marathon),
            None,
        )?;
        ensure!(
            !manager.package_paths.is_empty(),
            "No Marathon packages found"
        );
        Ok(Self { manager })
    }

    pub fn package_count(&self) -> usize {
        self.manager.package_paths.len()
    }

    pub fn worlds(&self) -> Vec<WorldEntry> {
        let mut worlds = self
            .manager
            .get_all_by_reference(deimos_data::map::SBubbleParent::ID.unwrap())
            .into_iter()
            .map(|(tag, _)| WorldEntry {
                tag: tag.0,
                name: self
                    .manager
                    .lookup
                    .named_tags
                    .iter()
                    .find(|e| e.hash == tag)
                    .map(|e| e.name.clone()),
            })
            .collect::<Vec<_>>();
        worlds.sort_unstable_by_key(|w| w.tag);
        worlds
    }

    pub fn world(&self, tag: u32) -> Result<World> {
        world::load(self, tag)
    }

    pub fn mesh(&self, key: MeshKey, layouts: &Layouts) -> Result<Mesh> {
        geometry::load(self, key, layouts)
    }

    pub fn materials(&self, owner: MaterialBinding) -> Result<MaterialSet> {
        materials::load(self, owner)
    }

    pub fn scene(&self, world: &World) -> Result<Scene> {
        scene::load(self, world)
    }

    pub fn named<T: TigerReadable>(&self, name: &str) -> Result<T> {
        Ok(self.manager.read_named_tag_struct(name)?)
    }

    pub fn read(&self, tag: u32) -> Result<Vec<u8>> {
        ensure!(tag != 0 && tag != u32::MAX, "Null tag {tag:08X}");
        self.manager
            .read_tag(tag)
            .with_context(|| format!("Read tag {tag:08X}"))
    }

    pub fn reference(&self, tag: u32) -> Result<u32> {
        Ok(self
            .manager
            .get_entry(tag)
            .with_context(|| format!("Missing tag {tag:08X}"))?
            .reference)
    }

    pub fn read_type<T: TigerReadable>(&self, tag: u32) -> Result<T> {
        if let Some(class) = T::ID.filter(|id| *id != u32::MAX) {
            ensure!(
                self.reference(tag)? == class,
                "Tag {tag:08X}: expected class {class:08X}"
            );
        }
        read_at(&self.read(tag)?, 0)
            .with_context(|| format!("Decode {} {tag:08X}", std::any::type_name::<T>()))
    }

    pub fn resolve(&self, hash: WideHash) -> Result<u32> {
        ensure!(hash.is_some(), "Null wide reference");
        match hash {
            WideHash::Hash32(h) => Ok(h.0),
            WideHash::Hash64(h) => self
                .manager
                .lookup
                .tag64_entries
                .get(&h.0)
                .map(|e| e.hash32.0)
                .with_context(|| format!("Missing wide tag {:016X}", h.0)),
        }
    }

    pub fn follow<T: TigerReadable>(&self, tag: &TagRef<T>) -> Result<T> {
        self.read_type(tag.taghash().0)
    }
}

pub fn read_at<T: TigerReadable>(bytes: &[u8], offset: u64) -> Result<T> {
    ensure!(
        offset <= bytes.len() as u64,
        "Structure outside tag at {offset:#X}"
    );
    let mut cursor = Cursor::new(bytes);
    cursor.set_position(offset);
    Ok(T::read_ds(&mut cursor)?)
}
