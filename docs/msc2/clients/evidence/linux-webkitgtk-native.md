# Linux WebKitGTK native-renderer evidence

- Runner: Debian GNU/Linux 12
- WebKitGTK package version: 2.50.6-1~deb12u2
- Native driver: /usr/bin/WebKitWebDriver (Usage: /usr/bin/WebKitWebDriver options)
- Tauri driver: /home/runner/.cargo/bin/tauri-driver
- Display server: Xvfb :99
- Screenshot: [linux-webkitgtk-native.png](linux-webkitgtk-native.png)

The production Svelte bundle was built into the debug Tauri binary and driven
through Tauri's Linux WebDriver bridge to the system WebKitGTK renderer. The
run covers the visible shell, navigation, CSS layout, dialog, deterministic
mutation, console view, deep link, fresh-profile entry, and reduced-motion
fallback.
