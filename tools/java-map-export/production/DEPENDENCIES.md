# Capture adapter dependencies

MSC Map Capture source and the distributed helper JARs are Apache-2.0 licensed;
see the adjacent LICENSE. The helper JARs contain MSC code only. Minecraft,
loader distributions, third-party mods, resource packs and account data are
not included in the MSC helper payload.

The matching local client supplies Minecraft and its libraries, Forge or
NeoForge (LGPL-2.1), or Fabric Loader and Fabric API (Apache-2.0), and Gson
(Apache-2.0). Their licenses remain with the client's own distributions.
Gradle and the loader build plugins are build tools; they are not packaged
inside these helper JARs. Exact build and runtime pins are recorded in
helpers.json. These adapters do not grant redistribution rights for any
third-party client content.
