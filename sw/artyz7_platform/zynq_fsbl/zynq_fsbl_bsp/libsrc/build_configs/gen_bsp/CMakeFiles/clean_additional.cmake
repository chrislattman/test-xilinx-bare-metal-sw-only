# Additional clean files
cmake_minimum_required(VERSION 3.16)

if("${CONFIG}" STREQUAL "" OR "${CONFIG}" STREQUAL "")
  file(REMOVE_RECURSE
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/diskio.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/ff.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/ffconf.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/sleep.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/xilffs.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/xilffs_config.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/xilrsa.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/xiltimer.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/include/xtimer_config.h"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/lib/libxilffs.a"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/lib/libxilrsa.a"
  "/home/chris/test-xilinx-bare-metal-sw-only/sw/artyz7_platform/zynq_fsbl/zynq_fsbl_bsp/lib/libxiltimer.a"
  )
endif()
