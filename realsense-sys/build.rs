use anyhow::Result;
use lazy_static::lazy_static;
use std::{
    collections::HashSet,
    fs::File,
    io::{prelude::*, BufReader},
    path::{Path, PathBuf},
    env,
};

lazy_static! {
    static ref CARGO_MANIFEST_DIR: PathBuf =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
}

fn main() -> Result<()> {
    if cfg!(feature = "doc-only") {
        return Ok(());
    }

    env::set_var("VCPKGRS_DYNAMIC", "1");

    // Probe libary
    let library = probe_library("realsense2")?;

    // Verify version
    let (mut include_dir, version) = library
        .include_paths
        .iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .filter_map(|path| {
            let dir = Path::new(path).join("librealsense2");
            if dir.is_dir() {
                match get_version_from_header_dir(&dir) {
                    Some(version) => Some((dir, version)),
                    None => None,
                }
            } else {
                None
            }
        })
        .next()
        .expect("fail to detect librealsense2 version");

    assert_eq!(
        &version.major,
        "2",
        "librealsense2 version {} is not supported",
        version.to_string()
    );

    // generate bindings
    #[cfg(feature = "buildtime-bindgen")]
    {
        let bindings = bindgen::Builder::default()
            .clang_arg("-fno-inline-functions")
            .header(include_dir.join("rs.h").to_str().unwrap())
            .clang_arg(format!("-I{}", include_dir.parent().unwrap().to_str().unwrap()))
            .header(
                include_dir
                    .join("h")
                    .join("rs_pipeline.h")
                    .to_str()
                    .unwrap(),
            )
            .header(
                include_dir
                    .join("h")
                    .join("rs_advanced_mode_command.h")
                    .to_str()
                    .unwrap(),
            )
            .header(include_dir.join("h").join("rs_config.h").to_str().unwrap())
            .header(
                CARGO_MANIFEST_DIR
                    .join("c")
                    .join("rsutil_delegate.h")
                    .to_str()
                    .unwrap(),
            )
            .whitelist_var("RS2_.*")
            .whitelist_type("rs2_.*")
            .whitelist_function("rs2_.*")
            .whitelist_function("_rs2_.*")
            .generate()
            .expect("Unable to generate bindings");

        // Write the bindings to file
        let bindings_dir = CARGO_MANIFEST_DIR.join("bindings");
        let bindings_file = bindings_dir.join("bindings.rs");

        std::fs::create_dir_all(&bindings_dir)?;
        bindings
            .write_to_file(bindings_file)
            .expect("Couldn't write bindings!");
    }

    #[cfg(target_env="msvc")]
    {
        include_dir.pop();
    }

    // compile and link rsutil_delegate.h statically
    cc::Build::new()
        .include(&include_dir)
        .include(CARGO_MANIFEST_DIR.join("c"))
        .file(CARGO_MANIFEST_DIR.join("c").join("rsutil_delegate.c"))
        .compile("rsutil_delegate");

    // link the librealsense2 shared library
    println!("cargo:rustc-link-lib=realsense2");

    Ok(())
}

fn get_version_from_header_dir<P>(dir: P) -> Option<Version>
where
    P: AsRef<Path>,
{
    let header_path = dir.as_ref().join("rs.h");

    let mut major_opt: Option<String> = None;
    let mut minor_opt: Option<String> = None;
    let mut patch_opt: Option<String> = None;
    let mut build_opt: Option<String> = None;

    let mut reader = BufReader::new(File::open(header_path).ok()?);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return None,
            _ => (),
        }

        const PREFIX: &str = "#define RS2_API_";
        if line.starts_with(PREFIX) {
            let mut tokens = line[PREFIX.len()..].split_whitespace();
            let name_opt = tokens.next();
            let version_opt = tokens.next();

            if let (Some(name), Some(version)) = (name_opt, version_opt) {
                let version_owned = version.to_owned();
                match name {
                    "MAJOR_VERSION" => major_opt = Some(version_owned),
                    "MINOR_VERSION" => minor_opt = Some(version_owned),
                    "PATCH_VERSION" => patch_opt = Some(version_owned),
                    "BUILD_VERSION" => build_opt = Some(version_owned),
                    _ => (),
                }
            }
        }

        if let (Some(major), Some(minor), Some(patch), Some(build)) =
            (&major_opt, &minor_opt, &patch_opt, &build_opt)
        {
            let version = Version {
                major: major.to_owned(),
                minor: minor.to_owned(),
                patch: patch.to_owned(),
                build: build.to_owned(),
            };
            return Some(version);
        }
    }
}

#[cfg(not(target_env="msvc"))]
fn probe_library(pkg_name: &str) -> Result<Library> {
    let package = pkg_config::probe_library(pkg_name)?;
    let lib = Library {
        pkg_name: pkg_name.to_owned(),
        libs: package.libs,
        link_paths: package.link_paths,
        framework_paths: package.framework_paths,
        include_paths: package.include_paths,
        version: package.version,
        prefix: PathBuf::from(pkg_config::get_variable(pkg_name, "prefix")?),
        libdir: PathBuf::from(pkg_config::get_variable(pkg_name, "libdir")?),
    };
    Ok(lib)
}

#[cfg(target_env="msvc")]
fn probe_library(pkg_name: &str) -> Result<Library> {
    let package = vcpkg::find_package(pkg_name)?;
    let lib = Library {
        pkg_name: pkg_name.to_owned(),
        libs: Vec::new(),
        link_paths: package.link_paths,
        framework_paths: Vec::new(),
        include_paths: package.include_paths,
        version: "2.38.1".to_string(),
        prefix: PathBuf::new(),
        libdir: PathBuf::new(),
    };
    Ok(lib)
}

#[derive(Debug, Clone)]
struct Version {
    major: String,
    minor: String,
    patch: String,
    build: String,
}

impl ToString for Version {
    fn to_string(&self) -> String {
        let Self {
            major,
            minor,
            patch,
            build,
        } = self;
        format!("{}.{}.{}.{}", major, minor, patch, build)
    }
}

#[derive(Debug)]
struct Library {
    pub pkg_name: String,
    pub libs: Vec<String>,
    pub link_paths: Vec<PathBuf>,
    pub framework_paths: Vec<PathBuf>,
    pub include_paths: Vec<PathBuf>,
    pub version: String,
    pub prefix: PathBuf,
    pub libdir: PathBuf,
}
