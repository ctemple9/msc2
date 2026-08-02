#!/usr/bin/env python3
"""Verify headless agent artifacts do not link desktop GUI frameworks."""

from __future__ import annotations

import argparse
import pathlib
import struct
import sys


EXPECTED_ARTIFACTS = {
    "macos": "macos/msc",
    "linux": "linux/msc",
    "windows": "windows/msc.exe",
}

MACHO_GUI_MARKERS = {
    "appkit",
    "applicationservices",
    "carbon",
    "coregraphics",
    "quartz",
    "quartzcore",
    "swiftui",
    "uikit",
    "webkit",
}

ELF_GUI_MARKERS = {
    "libgdk",
    "libgtk",
    "libqt",
    "libwayland",
    "libwx_gtk",
    "libx11",
    "libxcb",
    "libxcomposite",
    "libxcursor",
    "libxext",
    "libxi",
    "libxinerama",
    "libxkbcommon",
    "libxrandr",
    "libxrender",
}

PE_GUI_DLLS = {
    "comctl32.dll",
    "comdlg32.dll",
    "dwmapi.dll",
    "gdi32.dll",
    "uxtheme.dll",
    "user32.dll",
}

IMAGE_SUBSYSTEM_WINDOWS_CUI = 3
IMAGE_NT_OPTIONAL_HDR32_MAGIC = 0x10B
IMAGE_NT_OPTIONAL_HDR64_MAGIC = 0x20B


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--all-artifacts", type=pathlib.Path, required=True)
    return parser.parse_args()


def fail(message: str) -> None:
    raise SystemExit(message)


def read_c_string(blob: bytes, start: int) -> str:
    end = blob.find(b"\x00", start)
    if end == -1:
        fail("unterminated string in binary metadata")
    return blob[start:end].decode("utf-8", errors="replace")


def parse_macho_libraries(path: pathlib.Path) -> list[str]:
    blob = path.read_bytes()
    if len(blob) < 32:
        fail(f"{path}: Mach-O file is too small")

    magic = struct.unpack_from("<I", blob, 0)[0]
    if magic == 0xFEEDFACF:
        endian = "<"
        is_64 = True
    elif magic == 0xCFFAEDFE:
        endian = ">"
        is_64 = True
    elif magic == 0xFEEDFACE:
        endian = "<"
        is_64 = False
    elif magic == 0xCEFAEDFE:
        endian = ">"
        is_64 = False
    else:
        fail(f"{path}: expected a Mach-O binary")

    header_size = 32 if is_64 else 28
    _, _, _, _, ncmds, sizeofcmds, _ = struct.unpack_from(f"{endian}IiiIIII", blob, 0)
    offset = header_size
    end_of_commands = offset + sizeofcmds
    if end_of_commands > len(blob):
        fail(f"{path}: load commands exceed file size")

    libraries: list[str] = []
    dylib_commands = {0xC, 0x18, 0x1F, 0x23}
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from(f"{endian}II", blob, offset)
        if cmdsize < 8 or offset + cmdsize > len(blob):
            fail(f"{path}: invalid Mach-O load command size")
        if cmd in dylib_commands:
            name_offset = struct.unpack_from(f"{endian}I", blob, offset + 8)[0]
            start = offset + name_offset
            if start >= offset + cmdsize:
                fail(f"{path}: invalid Mach-O dylib name offset")
            libraries.append(read_c_string(blob, start))
        offset += cmdsize
    return libraries


def parse_elf_needed_libraries(path: pathlib.Path) -> list[str]:
    blob = path.read_bytes()
    if len(blob) < 64 or blob[:4] != b"\x7fELF":
        fail(f"{path}: expected an ELF binary")

    elf_class = blob[4]
    if elf_class == 1:
        is_64 = False
    elif elf_class == 2:
        is_64 = True
    else:
        fail(f"{path}: unsupported ELF class {elf_class}")

    data_encoding = blob[5]
    if data_encoding == 1:
        endian = "<"
    elif data_encoding == 2:
        endian = ">"
    else:
        fail(f"{path}: unsupported ELF endianness {data_encoding}")

    if is_64:
        e_shoff = struct.unpack_from(f"{endian}Q", blob, 40)[0]
        e_shentsize = struct.unpack_from(f"{endian}H", blob, 58)[0]
        e_shnum = struct.unpack_from(f"{endian}H", blob, 60)[0]
        section_fmt = f"{endian}IIQQQQIIQQ"
        section_size = 64
        dynamic_fmt = f"{endian}QQ"
    else:
        e_shoff = struct.unpack_from(f"{endian}I", blob, 32)[0]
        e_shentsize = struct.unpack_from(f"{endian}H", blob, 46)[0]
        e_shnum = struct.unpack_from(f"{endian}H", blob, 48)[0]
        section_fmt = f"{endian}IIIIIIIIII"
        section_size = 40
        dynamic_fmt = f"{endian}II"

    if e_shentsize != section_size:
        fail(f"{path}: unexpected ELF section size")
    if e_shoff + e_shentsize * e_shnum > len(blob):
        fail(f"{path}: section table exceeds file size")

    sections = []
    for index in range(e_shnum):
        start = e_shoff + index * e_shentsize
        sections.append(struct.unpack_from(section_fmt, blob, start))

    e_shstrndx = struct.unpack_from(f"{endian}H", blob, 62 if is_64 else 50)[0]
    shstr_section = sections[e_shstrndx]
    shstr_offset = shstr_section[4] if is_64 else shstr_section[4]
    shstr_size = shstr_section[5] if is_64 else shstr_section[5]
    shstr = blob[shstr_offset : shstr_offset + shstr_size]

    dynamic_section = None
    dynstr_section = None
    for section in sections:
        name_offset = section[0]
        name = read_c_string(shstr, name_offset)
        if name == ".dynamic":
            dynamic_section = section
        elif name == ".dynstr":
            dynstr_section = section

    if dynamic_section is None and dynstr_section is None:
        return []
    if dynamic_section is None or dynstr_section is None:
        fail(f"{path}: incomplete ELF dynamic sections")

    dyn_offset = dynamic_section[4]
    dyn_size = dynamic_section[5]
    dyn_entsize = dynamic_section[9]
    if dyn_entsize == 0:
        dyn_entsize = struct.calcsize(dynamic_fmt)
    dynstr_offset = dynstr_section[4]
    dynstr_size = dynstr_section[5]
    dynstr = blob[dynstr_offset : dynstr_offset + dynstr_size]

    needed: list[str] = []
    for entry_offset in range(dyn_offset, dyn_offset + dyn_size, dyn_entsize):
        tag, value = struct.unpack_from(dynamic_fmt, blob, entry_offset)
        if tag == 0:
            break
        if tag == 1:
            needed.append(read_c_string(dynstr, value))
    return needed


def parse_pe_subsystem_and_imports(path: pathlib.Path) -> tuple[int, list[str]]:
    blob = path.read_bytes()
    if len(blob) < 0x100 or blob[:2] != b"MZ":
        fail(f"{path}: expected a PE binary")

    pe_offset = struct.unpack_from("<I", blob, 0x3C)[0]
    if blob[pe_offset : pe_offset + 4] != b"PE\x00\x00":
        fail(f"{path}: missing PE signature")

    machine, section_count, _, _, _, optional_header_size, _ = struct.unpack_from(
        "<HHIIIHH", blob, pe_offset + 4
    )
    del machine
    optional_offset = pe_offset + 24
    magic = struct.unpack_from("<H", blob, optional_offset)[0]
    if magic not in {IMAGE_NT_OPTIONAL_HDR32_MAGIC, IMAGE_NT_OPTIONAL_HDR64_MAGIC}:
        fail(f"{path}: unsupported PE optional header")

    subsystem = struct.unpack_from("<H", blob, optional_offset + 68)[0]
    data_directory_offset = optional_offset + (96 if magic == IMAGE_NT_OPTIONAL_HDR32_MAGIC else 112)
    import_rva, import_size = struct.unpack_from("<II", blob, data_directory_offset + 8)

    section_offset = optional_offset + optional_header_size
    sections = []
    for index in range(section_count):
        start = section_offset + index * 40
        name, virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from(
            "<8sIIII", blob, start
        )
        sections.append(
            {
                "name": name.rstrip(b"\x00").decode("ascii", errors="replace"),
                "virtual_size": virtual_size,
                "virtual_address": virtual_address,
                "raw_size": raw_size,
                "raw_pointer": raw_pointer,
            }
        )

    def rva_to_offset(rva: int) -> int:
        for section in sections:
            start_rva = section["virtual_address"]
            span = max(section["virtual_size"], section["raw_size"])
            if start_rva <= rva < start_rva + span:
                return section["raw_pointer"] + (rva - start_rva)
        fail(f"{path}: could not map RVA 0x{rva:x} to file offset")

    if import_rva == 0 or import_size == 0:
        return subsystem, []

    imports: list[str] = []
    descriptor_offset = rva_to_offset(import_rva)
    while True:
        original_first_thunk, _, _, name_rva, _ = struct.unpack_from("<IIIII", blob, descriptor_offset)
        if original_first_thunk == 0 and name_rva == 0:
            break
        imports.append(read_c_string(blob, rva_to_offset(name_rva)))
        descriptor_offset += 20
    return subsystem, imports


def verify_macos(path: pathlib.Path) -> None:
    libraries = parse_macho_libraries(path)
    offenders = [lib for lib in libraries if any(marker in lib.lower() for marker in MACHO_GUI_MARKERS)]
    if offenders:
        fail(f"{path}: GUI-linked Mach-O frameworks found: {', '.join(offenders)}")


def verify_linux(path: pathlib.Path) -> None:
    libraries = parse_elf_needed_libraries(path)
    offenders = [lib for lib in libraries if any(marker in lib.lower() for marker in ELF_GUI_MARKERS)]
    if offenders:
        fail(f"{path}: desktop-linked ELF libraries found: {', '.join(offenders)}")


def verify_windows(path: pathlib.Path) -> None:
    subsystem, imports = parse_pe_subsystem_and_imports(path)
    if subsystem != IMAGE_SUBSYSTEM_WINDOWS_CUI:
        fail(f"{path}: expected Windows CUI subsystem, found {subsystem}")
    offenders = [dll for dll in imports if dll.lower() in PE_GUI_DLLS]
    if offenders:
        fail(f"{path}: GUI-linked PE imports found: {', '.join(offenders)}")


def main() -> int:
    args = parse_args()
    base_dir = args.all_artifacts
    if not base_dir.exists():
        fail(f"{base_dir}: artifact directory does not exist")

    verifiers = {
        "macos": verify_macos,
        "linux": verify_linux,
        "windows": verify_windows,
    }

    for platform, relative_path in EXPECTED_ARTIFACTS.items():
        artifact = base_dir / relative_path
        if not artifact.is_file():
            fail(f"missing {platform} artifact: {artifact}")
        verifiers[platform](artifact)
        print(f"ok {platform} {artifact}")

    print("ok all 3")
    return 0


if __name__ == "__main__":
    sys.exit(main())
