//! Enumerate and open children relative to the retained directory handle.
//! No child data lookup depends on an ancestor pathname remaining unchanged.
use super::{check_kind, portable_name};
use std::{
    fs::File,
    io,
    mem::{offset_of, size_of, zeroed},
    os::windows::io::{AsRawHandle, FromRawHandle},
    ptr,
};
use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT,
            FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile,
        },
    },
    Win32::{
        Foundation::{ERROR_NO_MORE_FILES, RtlNtStatusToDosError, UNICODE_STRING},
        Storage::FileSystem::{
            FILE_ATTRIBUTE_DIRECTORY, FILE_GENERIC_READ, FILE_ID_BOTH_DIR_INFO, FILE_SHARE_READ,
            FileIdBothDirectoryInfo, FileIdBothDirectoryRestartInfo, GetFileInformationByHandleEx,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

pub(crate) fn child(root: &File, name: &str, directory: bool) -> io::Result<File> {
    let mut wide: Vec<u16> = name.encode_utf16().collect();
    let bytes =
        u16::try_from(wide.len() * 2).map_err(|_| io::Error::other("source name is too long"))?;
    let mut name = UNICODE_STRING {
        Length: bytes,
        MaximumLength: bytes,
        Buffer: wide.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: root.as_raw_handle().cast(),
        ObjectName: &mut name,
        Attributes: 0,
        SecurityDescriptor: ptr::null_mut(),
        SecurityQualityOfService: ptr::null_mut(),
    };
    let mut handle = ptr::null_mut();
    let mut status: IO_STATUS_BLOCK = unsafe { zeroed() };
    // SAFETY: the root owns a live directory handle; name and every output
    // remain alive through this synchronous open. FILE_OPEN cannot create data.
    let result = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_GENERIC_READ,
            &attributes,
            &mut status,
            ptr::null(),
            0,
            FILE_SHARE_READ,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT
                | FILE_SYNCHRONOUS_IO_NONALERT
                | if directory {
                    FILE_DIRECTORY_FILE
                } else {
                    FILE_NON_DIRECTORY_FILE
                },
            ptr::null(),
            0,
        )
    };
    if result < 0 {
        return Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(result) } as i32,
        ));
    }
    // SAFETY: successful NtCreateFile transfers one live, non-inherited handle.
    let file = unsafe { File::from_raw_handle(handle) };
    check_kind(&file, directory)?;
    Ok(file)
}

pub(super) fn children(
    root: &File,
    visit: impl FnMut(&str, File, bool) -> io::Result<()>,
) -> io::Result<()> {
    children_with_names(root, true, visit)
}

pub(super) fn storage_children(
    root: &File,
    visit: impl FnMut(&str, File, bool) -> io::Result<()>,
) -> io::Result<()> {
    children_with_names(root, false, visit)
}

fn children_with_names(
    root: &File,
    portable: bool,
    mut visit: impl FnMut(&str, File, bool) -> io::Result<()>,
) -> io::Result<()> {
    // Eight-byte alignment covers FILE_ID_BOTH_DIR_INFO. Enumeration retains
    // one fixed-size buffer per traversal depth, never an all-entry list.
    let mut buffer = vec![0u64; 4096];
    let capacity = buffer.len() * size_of::<u64>();
    let mut restart = true;
    loop {
        let class = if restart {
            FileIdBothDirectoryRestartInfo
        } else {
            FileIdBothDirectoryInfo
        };
        restart = false;
        let result = unsafe {
            GetFileInformationByHandleEx(
                root.as_raw_handle().cast(),
                class,
                buffer.as_mut_ptr().cast(),
                capacity as u32,
            )
        };
        if result == 0 {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                Ok(())
            } else {
                Err(error)
            };
        }
        let mut offset = 0usize;
        loop {
            if offset + size_of::<FILE_ID_BOTH_DIR_INFO>() > capacity {
                return Err(io::Error::other("invalid directory enumeration"));
            }
            // SAFETY: Windows writes aligned directory records inside buffer;
            // lengths and offsets are checked before reading variable names.
            let entry = unsafe {
                &*buffer
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<FILE_ID_BOTH_DIR_INFO>()
            };
            let length = entry.FileNameLength as usize;
            let name_offset = offset + offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
            if length == 0 || !length.is_multiple_of(2) || name_offset + length > capacity {
                return Err(io::Error::other("invalid directory name"));
            }
            let units = unsafe {
                std::slice::from_raw_parts(
                    buffer.as_ptr().cast::<u8>().add(name_offset).cast::<u16>(),
                    length / 2,
                )
            };
            let name = String::from_utf16(units)
                .map_err(|_| io::Error::other("source names must be valid text"))?;
            if name != "." && name != ".." {
                if portable {
                    portable_name(&name)?;
                }
                if name.contains(['/', '\\']) {
                    return Err(io::Error::other("invalid directory child name"));
                }
                let directory = entry.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0;
                let file = child(root, &name, directory)?;
                visit(&name, file, directory)?;
            }
            if entry.NextEntryOffset == 0 {
                break;
            }
            let step = entry.NextEntryOffset as usize;
            if step < size_of::<FILE_ID_BOTH_DIR_INFO>() || !step.is_multiple_of(8) {
                return Err(io::Error::other("invalid directory offset"));
            }
            offset = offset
                .checked_add(step)
                .filter(|n| *n < capacity)
                .ok_or_else(|| io::Error::other("directory offset exceeds buffer"))?;
        }
    }
}
