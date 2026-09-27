#!/usr/bin/env python3
"""Import an exact BDS release's fresh-world gamerule enumeration.

Capture startup, `gamerule`, `help gamerule`, and clean shutdown in a disposable
world. Never capture defaults from an existing player's world.
"""
import argparse
import hashlib
import json
import pathlib
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--version', required=True)
parser.add_argument('--transcript', type=pathlib.Path, required=True)
parser.add_argument('--binary', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, default=pathlib.Path('crates/msc-infrastructure/data/gamerules'))
args = parser.parse_args()
text = args.transcript.read_text()
if not re.search(r'Version: ' + re.escape(args.version) + r'\s', text):
    raise ValueError('Transcript must identify the exact requested BDS release')
if 'CREATING VANILLA WORLD' not in text or 'Quit correctly' not in text or '/gamerule <rule: BoolGameRule>' not in text:
    raise ValueError('Need a fresh-world transcript including full gamerule help and clean shutdown')
build = re.search(r'Build ID: (\d+)', text)
line = next((line for line in text.splitlines() if line.count(' = ') >= 30), None)
if line is None:
    raise ValueError('No complete native enumeration found')
line = re.sub(r'^.*? INFO\] ', '', line)
pairs = [pair.split(' = ', 1) for pair in line.split(', ')]
if len({name.lower() for name, _ in pairs}) != len(pairs):
    raise ValueError('Duplicate native rule names')
existing_path = args.output / f'bedrock-{args.version}.json'
existing = json.loads(existing_path.read_text()) if existing_path.exists() else {}
metadata = {rule['id']: rule for rule in existing.get('rules', [])}
rules = []
for name, value in pairs:
    name = name.lower()
    if not re.fullmatch('[a-z][a-z0-9]*', name):
        raise ValueError(f'Unknown rule ID: {name}')
    if value in ('true', 'false'):
        kind, choices = 'boolean', []
    elif re.fullmatch(r'-?\d+', value):
        kind, choices = 'integer', []
    elif name == 'playerwaypoints' and value in ('everyone', 'off'):
        kind, choices = 'choice', ['everyone', 'off']
    else:
        raise ValueError(f'Unknown rule value schema: {name}={value}; inspect native help first')
    old = metadata.get(name, {})
    rules.append(dict(id=name, label=old.get('label', name),
                      description=old.get('description', f'Changes {name} for this world.'),
                      type=kind, defaultValue=value, choices=choices, experimental=False))
payload = dict(serverType='bedrock', minecraftVersion=args.version, complete=True,
               source=f'BDS {args.version} built-in gamerule enumeration; build {build[1] if build else "unknown"}',
               sourceSha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),
               rules=sorted(rules, key=lambda rule: rule['id']))
args.output.mkdir(parents=True, exist_ok=True)
existing_path.write_text(json.dumps(payload, indent=2) + '\n')
bundled = [json.loads(path.read_text()) for path in sorted(args.output.glob('*.json')) if path.name != 'catalogs.json']
(args.output / 'catalogs.json').write_text(json.dumps(bundled, indent=2) + '\n')
print(args.version, len(rules), 'registered rules')
