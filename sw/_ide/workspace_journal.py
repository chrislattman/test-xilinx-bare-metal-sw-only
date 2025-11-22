# 2025-11-22T17:22:07.436494700
import vitis

client = vitis.create_client()
client.set_workspace(path="sw")

platform = client.get_component(name="artyz7_platform")
status = platform.build()

vitis.dispose()

comp = client.get_component(name="hello_world")
comp.build()

vitis.dispose()

platform = client.get_component(name="artyz7_platform")
status = platform.build()

comp = client.get_component(name="hello_world")
comp.build()

