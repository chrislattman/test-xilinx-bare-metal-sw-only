fn main() {
    xil_bsp_build::Bsp {
        default_dir: "../../artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp",
        dir_env: "XIL_A9_0_BSP_DIR",
        // Must match TOOLCHAIN_C_FLAGS in the BSP's cortexa9_toolchain.cmake
        defines: &[],
    }
    .build();
}
