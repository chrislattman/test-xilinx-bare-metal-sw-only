use std::env;
use std::path::PathBuf;

fn main() {
    // From xil-a9-1-sys (`links = "xil_a9_1"`): the BSP's Xilinx.spec swaps GCC's
    // startup files for the ones the BSP's boot code expects, like in Vitis
    let specs = env::var("DEP_XIL_A9_1_SPECS").unwrap();
    println!("cargo:rustc-link-arg-bins=-specs={specs}");

    // This app's linker script (memory map), copied from the C application's
    // hello_world_1/src/lscript.ld. Its DDR regions must match rust_apps_libs/shared_mem
    let linker_script = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("linker.ld");
    println!("cargo:rerun-if-changed={}", linker_script.display());
    println!("cargo:rustc-link-arg-bins=-Wl,-T,{}", linker_script.display());
}
