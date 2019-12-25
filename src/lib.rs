use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

/// Meta-information of a cargo project.
pub struct Metadata {
    raw_toml_map: HashMap<cargo_metadata::PackageId, toml::Value>,
    metadata: cargo_metadata::Metadata,
}

#[derive(Copy, Clone, Debug)]
enum SemVer {
    Major,
    Minor,
    Patch,
}

impl Metadata {
    /// Create a `Metadata` a project at the given directory.
    pub fn from_dir<P: AsRef<Path>>(dir: P) -> io::Result<Self> {
        let metadata = cargo_metadata::MetadataCommand::new()
            .current_dir(dir)
            .exec()
            .map_err(io_error_other)?;

        let raw_toml_map = metadata
            .packages
            .iter()
            .map(|p| {
                let content = fs::read_to_string(&p.manifest_path)?;
                let raw_value = content.parse::<toml::Value>().map_err(io_error_other);
                raw_value.map(|v| (p.id.clone(), v))
            })
            .collect::<Result<_, _>>()?;

        Ok(Metadata {
            metadata,
            raw_toml_map,
        })
    }

    /// Bump the patch version of all packages.
    pub fn bump_all_patch_versions(&mut self) {
        for (_, mut raw_map) in &mut self.raw_toml_map {
            if let Some(version) = Self::get_version_mut(&mut raw_map) {
                let mut ver = semver::Version::parse(version.as_str().unwrap()).unwrap();
                ver.increment_patch();
                *version = toml::Value::String(ver.to_string());
            }
        }
    }

    /// Bump the patch version of the package with the given name.
    /// Return the new version.
    pub fn bump_patch_version(&mut self, name: &str) -> Option<semver::Version> {
        self.bump_version_inner(name, SemVer::Patch)
    }

    /// Bump the minor version of the package with the given name.
    /// Return the new version.
    pub fn bump_minor_version(&mut self, name: &str) -> Option<semver::Version> {
        self.bump_version_inner(name, SemVer::Minor)
    }

    /// Bump the minor version of the package with the given name.
    /// Return the new version.
    pub fn bump_major_version(&mut self, name: &str) -> Option<semver::Version> {
        self.bump_version_inner(name, SemVer::Major)
    }

    fn bump_version_inner(&mut self, name: &str, bump: SemVer) -> Option<semver::Version> {
        self.package_mut(name)
            .and_then(|raw_package| raw_package.get_mut("version"))
            .and_then(|raw_version| Self::bump_raw_version(raw_version, bump))
    }

    fn package_mut(&mut self, name: &str) -> Option<&mut toml::value::Table> {
        // `clone` to work around the borrow checker :(
        let package_id = self.package_id(name)?.clone();

        self.raw_toml_map
            .get_mut(&package_id)?
            .as_table_mut()?
            .get_mut("package")?
            .as_table_mut()
    }

    /// Return a package id of the package with the given name.
    fn package_id(&self, name: &str) -> Option<&cargo_metadata::PackageId> {
        self.metadata
            .packages
            .iter()
            .find(|p| p.name == name)
            .map(|p| &p.id)
    }

    // #[package]
    // version = "0.1"
    fn get_version_mut(table: &mut toml::Value) -> Option<&mut toml::Value> {
        table
            .as_table_mut()?
            .get_mut("package")?
            .as_table_mut()?
            .get_mut("version")
    }

    fn bump_raw_version(version: &mut toml::Value, bump: SemVer) -> Option<semver::Version> {
        let mut ver = semver::Version::parse(version.as_str()?).ok()?;
        match bump {
            SemVer::Major => ver.increment_major(),
            SemVer::Minor => ver.increment_minor(),
            SemVer::Patch => ver.increment_patch(),
        }
        *version = toml::Value::String(ver.to_string());
        Some(ver)
    }

    /// Write back the updated Cargo.toml.
    pub fn dump(&mut self) -> io::Result<()> {
        for p in &self.metadata.packages {
            let mut f = fs::File::create(&p.manifest_path)?;
            let raw_data = self.raw_toml_map.get(&p.id).unwrap();
            f.write_all(
                toml::to_string(raw_data)
                    .map_err(io_error_other)?
                    .as_bytes(),
            )?;
        }

        Ok(())
    }
}

fn io_error_other<E>(error: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::Other, error.into())
}
