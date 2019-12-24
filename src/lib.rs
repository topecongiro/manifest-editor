use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;

pub struct Metadata {
    raw_toml_map: HashMap<cargo_metadata::PackageId, toml::Value>,
    metadata: cargo_metadata::Metadata,
}

impl Metadata {
    /// Create a `Metadata` for the given directory.
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
                let raw_value = content
                    .parse::<toml::Value>()
                    .map_err(io_error_other);
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

    // #[package]
    // version = "0.1"
    fn get_version_mut(table: &mut toml::Value) -> Option<&mut toml::Value> {
        table
            .as_table_mut()?
            .get_mut("package")?
            .as_table_mut()?
            .get_mut("version")
    }

    /// Dump the updated Cargo.toml.
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
