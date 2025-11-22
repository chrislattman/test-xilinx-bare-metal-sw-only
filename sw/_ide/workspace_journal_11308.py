# 2025-11-21T12:18:52.189756900
import vitis

client = vitis.create_client()
client.set_workspace(path="sw")

advanced_options = client.create_advanced_options_dict(dt_overlay="0")

platform = client.create_platform_component(name = "artyz7_platform",hw_design = "$COMPONENT_LOCATION/../../fpga/artyz7_wrapper.xsa",os = "standalone",cpu = "ps7_cortexa9_0",domain_name = "standalone_ps7_cortexa9_0",generate_dtb = False,advanced_options = advanced_options,compiler = "gcc")

comp = client.create_app_component(name="hello_world",platform = "$COMPONENT_LOCATION/../artyz7_platform/export/artyz7_platform/artyz7_platform.xpfm",domain = "standalone_ps7_cortexa9_0",template = "hello_world")

vitis.dispose()

