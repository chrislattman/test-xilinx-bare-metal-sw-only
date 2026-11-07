fn main() {
    xil_bsp_build::Bsp {
        default_dir: "../../artyz7_platform/ps7_cortexa9_1/standalone_ps7_cortexa9_1/bsp",
        dir_env: "XIL_A9_1_BSP_DIR",
        // Must match TOOLCHAIN_C_FLAGS in the BSP's cortexa9_toolchain.cmake
        defines: &[("USE_AMP", "1")],
    }
    .build();
}
