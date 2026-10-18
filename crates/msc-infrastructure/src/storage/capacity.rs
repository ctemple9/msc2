//! Display capacity only; resource admission keeps its existing free-space checks.
use std::{io, path::Path};

#[cfg(not(target_os = "macos"))]
pub fn available_space(path: &Path) -> io::Result<u64> {
    fs2::available_space(path)
}

#[cfg(target_os = "macos")]
pub fn available_space(path: &Path) -> io::Result<u64> {
    use std::{ffi::c_void, os::unix::ffi::OsStrExt, ptr};
    type Ref = *const c_void;
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFURLVolumeAvailableCapacityForImportantUsageKey: Ref;
        fn CFURLCreateFromFileSystemRepresentation(
            allocator: Ref,
            bytes: *const u8,
            length: isize,
            is_directory: u8,
        ) -> Ref;
        fn CFURLCopyResourcePropertyForKey(
            url: Ref,
            key: Ref,
            value: *mut Ref,
            error: *mut Ref,
        ) -> u8;
        fn CFGetTypeID(value: Ref) -> usize;
        fn CFNumberGetTypeID() -> usize;
        fn CFNumberGetValue(number: Ref, kind: isize, value: *mut c_void) -> u8;
        fn CFRelease(value: Ref);
    }
    // Copy/create functions transfer one reference; every exit releases it.
    struct Owned(Ref);
    impl Drop for Owned {
        fn drop(&mut self) {
            unsafe { CFRelease(self.0) }
        }
    }
    let unavailable = || io::Error::other("macOS available capacity is unavailable");
    let bytes = path.as_os_str().as_bytes();
    let length = isize::try_from(bytes.len()).map_err(|_| unavailable())?;
    // The URL owns a copy of these path bytes. The key is a framework constant;
    // copied properties are type-checked before interpreting their contents.
    unsafe {
        let url = CFURLCreateFromFileSystemRepresentation(ptr::null(), bytes.as_ptr(), length, 0);
        if url.is_null() {
            return Err(unavailable());
        }
        let url = Owned(url);
        let mut value = ptr::null();
        let success = CFURLCopyResourcePropertyForKey(
            url.0,
            kCFURLVolumeAvailableCapacityForImportantUsageKey,
            &mut value,
            ptr::null_mut(),
        );
        let value = (!value.is_null()).then(|| Owned(value));
        let Some(value) = value.filter(|v| success != 0 && CFGetTypeID(v.0) == CFNumberGetTypeID())
        else {
            return Err(unavailable());
        };
        let mut capacity: i64 = 0;
        // kCFNumberSInt64Type = 4, independent of C long width.
        if CFNumberGetValue(value.0, 4, (&mut capacity as *mut i64).cast()) == 0 {
            return Err(unavailable());
        }
        u64::try_from(capacity).map_err(|_| unavailable())
    }
}
