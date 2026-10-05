#!/usr/bin/env python3
"""Build (without launching Minecraft) and stage the pinned capture adapters."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
DRIVER = ROOT / 'tools/java-map-export/production/build.py'
spec = importlib.util.spec_from_file_location('map_capture_build', DRIVER)
build = importlib.util.module_from_spec(spec)
spec.loader.exec_module(build)


def verified(source):
    manifest = json.loads((source / 'helpers.json').read_text(encoding='utf-8'))
    if manifest.get('version') != 1 or manifest.get('buildSourceSha256') != build.source_digest():
        raise ValueError('capture helper source identity is stale')
    helpers = manifest.get('helpers', [])
    if len(helpers) != len(build.PINS) or {h['loaderFamily'] for h in helpers} != set(build.PINS):
        raise ValueError('complete Fabric, Forge and NeoForge helper set is required')
    files = ['helpers.json', 'LICENSE', 'DEPENDENCIES.md']
    for helper in helpers:
        family = helper['loaderFamily']
        if any(helper.get(key) != value for key, value in build.PINS[family].items()):
            raise ValueError(f'{family} helper does not match its pinned toolchain')
        name = f'msc-map-capture-{family}-0.2.0.jar'
        if helper.get('file') != name or helper.get('format') != 'msc-contextual-mesh-1' or helper.get('buildOnly') is not True or helper.get('gameLaunched') is not False:
            raise ValueError('unexpected capture helper name or format')
        path = source / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f'missing regular helper file: {name}')
        data = path.read_bytes()
        if not 0 < len(data) <= 8 * 1024 * 1024 or helper.get('bytes') != len(data) or helper.get('sha256') != hashlib.sha256(data).hexdigest():
            raise ValueError(f'capture helper checksum mismatch: {name}')
        files.append(name)
    for name in files:
        path = source / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f'missing regular capture payload file: {name}')
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path)
    parser.add_argument('--build-only', action='store_true')
    args = parser.parse_args()
    if not args.build_only and args.output_dir is None:
        parser.error('--output-dir or --build-only is required')
    source = Path(os.environ.get('MSC2_MAP_CAPTURE_HELPERS', ROOT / 'target/map-capture-helpers')).resolve()
    try:
        files = verified(source)
    except (OSError, ValueError, KeyError, TypeError):
        # An explicitly selected payload must already be valid; never replace it.
        if 'MSC2_MAP_CAPTURE_HELPERS' in os.environ:
            raise
        subprocess.run([sys.executable, str(DRIVER), '--output', str(source)], check=True)
        files = verified(source)
    if args.output_dir is not None:
        parent = args.output_dir.resolve() / 'map-capture'
        parent.mkdir(parents=True, exist_ok=True)
        destination = parent / '0.2.0'
        with tempfile.TemporaryDirectory(prefix='.capture-', dir=parent) as candidate:
            candidate = Path(candidate)
            for name in files:
                shutil.copyfile(source / name, candidate / name)
            if destination.is_symlink():
                raise ValueError('capture staging destination is a symbolic link')
            if destination.exists():
                shutil.rmtree(destination)
            candidate.rename(destination)
    print(f'Capture helper payload: {source}')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        print(f'Map capture staging failed: {error}', file=sys.stderr)
        sys.exit(1)
