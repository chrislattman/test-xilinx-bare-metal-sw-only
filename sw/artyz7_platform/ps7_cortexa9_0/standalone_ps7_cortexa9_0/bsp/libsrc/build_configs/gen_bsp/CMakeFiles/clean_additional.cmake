# Additional clean files
cmake_minimum_required(VERSION 3.16)

if("${CONFIG}" STREQUAL "" OR "${CONFIG}" STREQUAL "")
  file(REMOVE_RECURSE
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/diskio.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/ff.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/ffconf.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/sleep.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/xilffs.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/xilffs_config.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/xiltimer.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/include/xtimer_config.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/lib/libxilffs.a"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/ps7_cortexa9_0/standalone_ps7_cortexa9_0/bsp/lib/libxiltimer.a"
  )
endif()
