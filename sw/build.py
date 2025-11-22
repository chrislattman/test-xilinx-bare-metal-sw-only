import vitis

client = vitis.create_client()
client.update_workspace(".")

platform = client.get_component("artyz7_platform")
platform.build()

app = client.get_component("hello_world")
app.build()

vitis.dispose()
