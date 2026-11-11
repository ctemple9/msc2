fn main() {
    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if windows_msvc && std::env::var("PROFILE").as_deref() == Ok("debug") {
        // Unoptimized CLI futures exceed Windows' default 1 MiB startup stack
        // before the service dispatcher can connect. Reserve address space;
        // Windows commits pages on demand. Optimized release builds are unchanged.
        println!("cargo:rustc-link-arg-bin=msc=/STACK:16777216");
    }
}
