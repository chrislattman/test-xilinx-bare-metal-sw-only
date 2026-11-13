//! Build-script helper shared by the `xil-a9-*-sys` crates.
//!
//! Given a Vitis standalone BSP directory (the one holding `include/`, `lib/`
//! and `Xilinx.spec`), [`Bsp::build`]:
//!
//! 1. generates a wrapper header that includes every header in the BSP's
//!    `include/` directory (minus [`SKIP_HEADERS`]) and runs bindgen over it,
//!    exposing every declaration from those headers,
//! 2. compiles the C shims (macro wrappers from `c/xilrs_shim.c` plus the
//!    `static inline` wrappers bindgen generates) with the Vitis GCC,
//! 3. tells rustc how to link the BSP archives into the final binary,
//! 4. exports the BSP paths to dependent build scripts (`DEP_<LINKS>_*`).
//!
//! It runs on the host as a build-dependency, so it is not a default
//! workspace member (the workspace builds for `armv7a-none-eabihf` by default).

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SHIM_H: &str = include_str!("../c/xilrs_shim.h");
const SHIM_C: &str = include_str!("../c/xilrs_shim.c");

/// Prefix of the macro shims in `c/xilrs_shim.h`, stripped on the Rust side.
const SHIM_PREFIX: &str = "xilrs_";

/// Same CPU flags the Vitis BSP was compiled with (`cortexa9_toolchain.cmake`).
const CPU_FLAGS: &[&str] = &["-mcpu=cortex-a9", "-mfpu=vfpv3", "-mfloat-abi=hard"];

/// BSP headers that can't be parsed for this target, left out of the bindings.
pub const SKIP_HEADERS: &[&str] = &[
    // VxWorks-only (needs vxWorks.h)
    "xil_cache_vxworks.h",
    // Legacy _m macro compat layer; references XPmonPsv, which no driver here defines
    "xil_macroback.h",
];

/// BSP archives in the order Vitis links them (`hello_world/build/build.ninja`).
/// Missing ones (e.g. `xilffs` on A9#1) are skipped.
const BSP_LIBS: &[&str] = &["xilstandalone", "xiltimer", "xilffs", "xil"];

/// One standalone BSP domain.
pub struct Bsp<'a> {
    /// BSP directory, relative to the calling crate's manifest dir.
    pub default_dir: &'a str,
    /// Environment variable that overrides `default_dir`.
    pub dir_env: &'a str,
    /// Extra `-D` defines matching the BSP's `TOOLCHAIN_C_FLAGS`
    /// (e.g. `USE_AMP=1` for A9#1).
    pub defines: &'a [(&'a str, &'a str)],
}

impl Bsp<'_> {
    pub fn build(&self) {
        let target = env::var("TARGET").unwrap();
        if !target.starts_with("armv7a-none-eabihf") {
            panic!(
                "the Xilinx A9 BSP bindings only build for armv7a-none-eabihf (got `{target}`); \
                 build from the sw/ workspace or pass --target armv7a-none-eabihf"
            );
        }

        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        println!("cargo:rerun-if-env-changed={}", self.dir_env);
        let bsp_dir = env::var_os(self.dir_env)
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir.join(self.default_dir));
        let bsp_dir = bsp_dir.canonicalize().unwrap_or_else(|e| {
            panic!(
                "BSP directory {} not found ({e}); generate the platform in Vitis or set {}",
                bsp_dir.display(),
                self.dir_env
            )
        });
        let include_dir = bsp_dir.join("include");
        let lib_dir = bsp_dir.join("lib");

        let libs: Vec<&str> = BSP_LIBS
            .iter()
            .copied()
            .filter(|l| lib_dir.join(format!("lib{l}.a")).exists())
            .collect();
        if !libs.contains(&"xil") || !libs.contains(&"xilstandalone") {
            panic!(
                "{} has no libxil.a/libxilstandalone.a; build the platform (artyz7_platform) in Vitis first",
                lib_dir.display()
            );
        }
        // Relink dependents whenever the platform is rebuilt
        for l in &libs {
            println!("cargo:rerun-if-changed={}", lib_dir.join(format!("lib{l}.a")).display());
        }
        println!("cargo:rerun-if-changed={}", include_dir.display());
        let have_xilffs = libs.contains(&"xilffs");
        let headers = list_headers(&include_dir);

        let gcc = find_gcc();
        let sysroot = gcc_sysroot(&gcc);

        let mut defines: Vec<(String, String)> = vec![("SDT".into(), "1".into())];
        defines.extend(self.defines.iter().map(|(k, v)| (k.to_string(), v.to_string())));

        // The C sources are embedded in this crate so edits rebuild the sys crates
        let c_dir = out_dir.join("c");
        fs::create_dir_all(&c_dir).unwrap();
        fs::write(c_dir.join("xilrs_wrapper.h"), wrapper_header(&headers)).unwrap();
        fs::write(c_dir.join("xilrs_shim.h"), SHIM_H).unwrap();
        fs::write(c_dir.join("xilrs_shim.c"), SHIM_C).unwrap();

        let static_wrappers = out_dir.join("static_wrappers");
        generate_bindings(
            &c_dir,
            &include_dir,
            &sysroot,
            &defines,
            &static_wrappers,
            &out_dir.join("bindings.rs"),
        );

        // Shims: hand-written macro wrappers + bindgen's static inline wrappers
        let mut cc = cc::Build::new();
        cc.compiler(&gcc)
            .file(c_dir.join("xilrs_shim.c"))
            .file(static_wrappers.with_extension("c"))
            .include(&c_dir)
            .include(&include_dir)
            .flag("-ffunction-sections")
            .flag("-fdata-sections")
            .warnings(false);
        for f in CPU_FLAGS {
            cc.flag(f);
        }
        for (k, v) in &defines {
            cc.define(k, v.as_str());
        }
        cc.compile("xilrs_shim");

        // The BSP archives call each other and newlib (and newlib calls back into
        // libxilstandalone for _write/_sbrk/...), so they must be linked as one
        // --start-group/--end-group like Vitis does. A dependency's build script
        // can't pass raw linker args to the final binary, so the group lives in a
        // GNU ld input script disguised as an archive (the same trick glibc uses
        // for libc.so). The final link must use (Vitis) arm-none-eabi-gcc so
        // -lgcc/-lc resolve to the right multilib.
        let mut group = String::from("/* Generated by xil-bsp-build */\nGROUP (");
        for l in &libs {
            write!(group, " \"{}\"", lib_dir.join(format!("lib{l}.a")).display()).unwrap();
        }
        group.push_str(" -lgcc -lc )\n");
        fs::write(out_dir.join("libxilrs_bsp.a"), group).unwrap();
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        println!("cargo:rustc-link-lib=static:-bundle=xilrs_bsp");

        // Available to dependent build scripts as DEP_<LINKS>_<KEY>
        println!("cargo:bsp_dir={}", bsp_dir.display());
        println!("cargo:include={}", include_dir.display());
        println!("cargo:lib_dir={}", lib_dir.display());
        println!("cargo:specs={}", bsp_dir.join("Xilinx.spec").display());
        println!("cargo:gcc={}", gcc.display());
        println!("cargo:xilffs={}", have_xilffs as u8);
    }
}

/// Every `*.h` directly under the BSP's `include/`, sorted, minus [`SKIP_HEADERS`].
fn list_headers(include_dir: &Path) -> Vec<String> {
    let mut headers: Vec<String> = fs::read_dir(include_dir)
        .unwrap_or_else(|e| panic!("can't read {}: {e}", include_dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.ends_with(".h") && !SKIP_HEADERS.contains(&n.as_str()))
        .collect();
    headers.sort();
    headers
}

/// The umbrella header handed to bindgen.
fn wrapper_header(headers: &[String]) -> String {
    let mut h = String::from(
        "/* Generated by xil-bsp-build: every header in the BSP's include/ directory */\n\
         #ifndef XILRS_WRAPPER_H\n#define XILRS_WRAPPER_H\n\n\
         #include <stdlib.h> /* rand()/srand() (newlib libc) */\n\n",
    );
    for name in headers {
        writeln!(h, "#include \"{name}\"").unwrap();
    }
    h.push_str("\n#include \"xilrs_shim.h\"\n\n#endif\n");
    h
}

fn generate_bindings(
    c_dir: &Path,
    include_dir: &Path,
    sysroot: &Path,
    defines: &[(String, String)],
    static_wrappers: &Path,
    out: &Path,
) {
    let mut builder = bindgen::Builder::default()
        .header(c_dir.join("xilrs_wrapper.h").to_str().unwrap())
        // bindgen passes --target from $TARGET; the rest matches the BSP build
        .clang_args(CPU_FLAGS)
        // arm-none-eabi-gcc defaults to short enums (bare-metal AAPCS,
        // Tag_ABI_enum_size: small); clang uses int-sized enums for this target,
        // which would misplace every struct field after an enum
        .clang_arg("-fshort-enums")
        // Same as Vitis' -U__clang__: BSP headers that check __clang__ mean Arm
        // Compiler, but these were compiled with GCC, so take the __GNUC__ paths
        .clang_arg("-U__clang__")
        .clang_arg("-Wno-builtin-macro-redefined")
        // xbasic_types.h has a #warning that it's deprecated
        .clang_arg("-Wno-#warnings")
        .clang_arg(format!("--sysroot={}", sysroot.display()))
        .clang_arg(format!("-isystem{}", sysroot.join("usr/include").display()))
        .clang_arg(format!("-I{}", include_dir.display()))
        .clang_arg(format!("-I{}", c_dir.display()))
        .use_core()
        .ctypes_prefix("core::ffi")
        .rust_edition(bindgen::RustEdition::Edition2024)
        .default_enum_style(bindgen::EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        })
        .derive_default(true)
        .layout_tests(true)
        // Everything the BSP headers (and our shim) declare, plus whatever types
        // those declarations pull in from newlib
        .allowlist_file(format!("{}/.*", regex_escape(include_dir.to_str().unwrap())))
        .allowlist_file(regex_escape(c_dir.join("xilrs_shim.h").to_str().unwrap()))
        .allowlist_function("rand|srand")
        // static inline functions (Xil_In32/Xil_Out32, ...) get real C wrappers
        .wrap_static_fns(true)
        .wrap_static_fns_path(static_wrappers)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .parse_callbacks(Box::new(XilCallbacks));
    for (k, v) in defines {
        builder = builder.clang_arg(format!("-D{k}={v}"));
    }
    builder
        .generate()
        .expect("bindgen failed on the BSP headers")
        .write_to_file(out)
        .unwrap();
}

/// Renames shims and tweaks macro constant types
#[derive(Debug)]
struct XilCallbacks;

impl bindgen::callbacks::ParseCallbacks for XilCallbacks {
    fn generated_name_override(&self, info: bindgen::callbacks::ItemInfo<'_>) -> Option<String> {
        info.name.strip_prefix(SHIM_PREFIX).map(str::to_owned)
    }

    /// Driver functions return `int`/`s32` status codes, so make `XST_*` signed
    /// (bindgen would otherwise pick `u32` for `#define XST_SUCCESS 0L`).
    fn int_macro(&self, name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
        name.starts_with("XST_").then_some(bindgen::callbacks::IntKind::Int)
    }
}

/// `XIL_ARM_GCC`, else the Vitis toolchain under `$XILINX_VITIS`
/// (set by `settings64.sh`), else `arm-none-eabi-gcc` from `PATH`.
fn find_gcc() -> PathBuf {
    println!("cargo:rerun-if-env-changed=XIL_ARM_GCC");
    println!("cargo:rerun-if-env-changed=XILINX_VITIS");
    if let Some(p) = env::var_os("XIL_ARM_GCC") {
        return p.into();
    }
    let exe = if cfg!(windows) { "arm-none-eabi-gcc.exe" } else { "arm-none-eabi-gcc" };
    if let Some(vitis) = env::var_os("XILINX_VITIS") {
        let host = if cfg!(windows) { "nt" } else { "lin" };
        let p = Path::new(&vitis)
            .join("gnu/aarch32")
            .join(host)
            .join("gcc-arm-none-eabi/bin")
            .join(exe);
        if p.exists() {
            return p;
        }
    }
    // Only the Xilinx GCC works here: bindgen needs its newlib sysroot, and other
    // arm-none-eabi-gcc builds (e.g. Ubuntu's) don't report one
    let gcc = PathBuf::from(exe);
    let version = Command::new(&gcc).arg("--version").output().map(|o| o.stdout);
    if !version.is_ok_and(|v| String::from_utf8_lossy(&v).contains("xilinx")) {
        panic!(
            "Vitis' arm-none-eabi-gcc not found: set XILINX_VITIS (e.g. by sourcing Vitis \
             settings64.sh) or XIL_ARM_GCC. For rust-analyzer, set XILINX_VITIS in \
             rust-analyzer.cargo.extraEnv (.vscode/settings.json)"
        );
    }
    gcc
}

fn gcc_sysroot(gcc: &Path) -> PathBuf {
    let out = Command::new(gcc)
        .args(CPU_FLAGS)
        .arg("-print-sysroot")
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", gcc.display()));
    let s = String::from_utf8(out.stdout).unwrap();
    let p = PathBuf::from(s.trim());
    p.canonicalize().unwrap_or(p)
}

fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if "\\.+*?()|[]{}^$".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
