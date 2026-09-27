#!/usr/bin/env python3
"""Extract exact-release gamerule catalogs from Mojang's server bytecode.

Uses javap, without executing the downloaded Minecraft classes. Unknown bytecode
shapes fail closed. Artifacts stay in the supplied scratch cache, outside the repo.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import urllib.request
import zipfile

MANIFEST = 'https://piston-meta.mojang.com/mc/game/version_manifest_v2.json'


def download(asset, path):
    if not path.exists():
        urllib.request.urlretrieve(asset['url'], path)
    if hashlib.sha1(path.read_bytes()).hexdigest() != asset['sha1']:
        raise ValueError(f'SHA-1 mismatch: {path}')


def number(line):
    match = re.search(r'iconst_(m1|[0-5])', line)
    if match:
        return -1 if match[1] == 'm1' else int(match[1])
    match = re.search(r'(?:bipush|sipush)\s+(-?\d+)|// int (-?\d+)', line)
    return int(match[1] or match[2]) if match else None


def extract(version, manifest, cache, output):
    entry = next(v for v in manifest['versions'] if v['id'] == version)
    release = json.load(urllib.request.urlopen(entry['url']))
    directory = cache / version
    directory.mkdir(parents=True, exist_ok=True)
    download(release['downloads']['server'], directory / 'server.jar')
    with zipfile.ZipFile(directory / 'server.jar') as archive:
        inner = next(n for n in archive.namelist() if n.startswith('META-INF/versions/') and n.endswith('.jar'))
        (directory / 'inner.jar').write_bytes(archive.read(inner))
    modern = 'net.minecraft.world.level.gamerules.GameRules'
    legacy = 'net.minecraft.world.level.GameRules'
    mappings = release['downloads'].get('server_mappings')
    if mappings:
        download(mappings, directory / 'server_mappings.txt')
        text = (directory / 'server_mappings.txt').read_text()
        match = re.search(r'^(' + re.escape(modern) + '|' + re.escape(legacy) + r') -> (\S+):', text, re.M)
        if not match:
            raise ValueError('No GameRules class in official mappings')
        modern_layout = match[1] == modern
        classname = match[2]
    else:
        classname, modern_layout = modern, True
    bytecode = subprocess.check_output(['javap', '-p', '-c', '-classpath', str(directory / 'inner.jar'), classname], text=True)
    (directory / 'gamerules.txt').write_text(bytecode)
    download(release['downloads']['client'], directory / 'client.jar')
    with zipfile.ZipFile(directory / 'client.jar') as archive:
        language = json.loads(archive.read('assets/minecraft/lang/en_us.json'))
    aliases = {
        'show_advancement_messages': 'announceAdvancements', 'command_blocks_work': 'commandBlocksEnabled',
        'elytra_movement_check': 'disableElytraMovementCheck', 'player_movement_check': 'disablePlayerMovementCheck',
        'raids': 'disableRaids', 'advance_time': 'doDaylightCycle', 'advance_weather': 'doWeatherCycle',
        'entity_drops': 'doEntityDrops', 'immediate_respawn': 'doImmediateRespawn', 'spawn_phantoms': 'doInsomnia',
        'limited_crafting': 'doLimitedCrafting', 'mob_drops': 'doMobLoot', 'spawn_mobs': 'doMobSpawning',
        'spawn_patrols': 'doPatrolSpawning', 'block_drops': 'doTileDrops', 'spawn_wandering_traders': 'doTraderSpawning',
        'spread_vines': 'doVinesSpread', 'spawn_wardens': 'doWardenSpawning',
        'max_command_sequence_length': 'maxCommandChainLength', 'max_command_forks': 'maxCommandForkCount',
        'natural_health_regeneration': 'naturalRegeneration', 'max_snow_accumulation_height': 'snowAccumulationHeight',
        'respawn_radius': 'spawnRadius', 'spawner_blocks_work': 'spawnerBlocksEnabled',
        'max_block_modifications': 'commandModificationBlockLimit',
    }
    language_ids = {re.sub('[^a-z0-9]', '', key[9:].lower()): key for key in language
                    if key.startswith('gamerule.') and not key.endswith('.description')}
    body = bytecode.split('static {};', 1)[1]
    blocks = re.split(r'\n\s*\d+: ldc(?:_w)?\s+[^\n]*// String ', body)[1:]
    rules = []
    for block in blocks:
        name, _, instructions = block.partition('\n')
        name = name.strip()
        if not re.fullmatch(r'[a-zA-Z][a-zA-Z0-9_]*', name):
            raise ValueError(f'Unexpected rule ID {name}')
        calls = [line for line in instructions.splitlines() if 'invokestatic' in line and '// Method' in line]
        if modern_layout:
            call = next((line for line in calls if re.search(r'Method \w+:\(Ljava/lang/String;L[^;]+;[ZI]', line)), None)
            if not call:
                raise ValueError(f'Unrecognized registration for {name}')
            signature = call.split('// Method ', 1)[1]
            kind = 'boolean' if re.search(r';Z\)', signature) else 'integer'
            prefix = instructions.split(call, 1)[0]
        else:
            call = next((line for line in calls if re.search(r'Method [^:]+:\([ZI]', line)), None)
            if not call:
                raise ValueError(f'Unrecognized legacy type for {name}')
            kind = 'boolean' if re.search(r':\(Z', call) else 'integer'
            prefix = instructions.split(call, 1)[0]
        constants = [value for line in prefix.splitlines() if (value := number(line)) is not None]
        if not constants:
            raise ValueError(f'No default for {name}')
        rule = {'id': ('minecraft:' + name) if modern_layout else name, 'type': kind,
                'defaultValue': str(constants[0]).lower() if kind == 'integer' else str(bool(constants[0])).lower(),
                'choices': [], 'experimental': 'FeatureFlag' in prefix or (re.search(r'getstatic[^\n]*:L[^;]+;\n[^\n]*invokestatic', prefix) is not None)}
        if kind == 'integer' and modern_layout:
            if len(constants) < 2:
                raise ValueError(f'No integer bounds for {name}')
            rule['minimum'] = constants[1]
            rule['maximum'] = constants[2] if len(constants) > 2 else 2147483647
        direct = 'gamerule.minecraft.' + name
        legacy_name = aliases.get(name, name)
        key = direct if direct in language else language_ids.get(re.sub('[^a-z0-9]', '', legacy_name.lower()))
        words = re.sub(r'([a-z])([A-Z])', r'\1 \2', name).replace('_', ' ')
        rule['label'] = language.get(key, words.capitalize())
        rule['description'] = language.get(str(key) + '.description', 'Changes ' + words.lower() + ' for this world.')
        if not modern_layout and kind == 'integer' and re.search(r':\(II', call):
            if len(constants) < 2:
                raise ValueError(f'No legacy integer bounds for {name}')
            rule['minimum'] = constants[1]
            rule['maximum'] = constants[2] if len(constants) > 2 else 2147483647
        rules.append(rule)
    if len({r['id'] for r in rules}) != len(rules) or not 30 <= len(rules) <= 200:
        raise ValueError('Unexpected/incomplete registration list')
    payload = {'serverType': 'java', 'minecraftVersion': version, 'complete': True,
               'source': release['downloads']['server']['url'],
               'sourceSha1': release['downloads']['server']['sha1'], 'languageSource': release['downloads']['client']['url'], 'languageSha1': release['downloads']['client']['sha1'], 'rules': sorted(rules, key=lambda r: r['id'])}
    output.mkdir(parents=True, exist_ok=True)
    (output / f'java-{version}.json').write_text(json.dumps(payload, indent=2) + '\n')
    bundled = [json.loads(path.read_text()) for path in sorted(output.glob('*.json')) if path.name != 'catalogs.json']
    (output / 'catalogs.json').write_text(json.dumps(bundled, indent=2) + '\n')
    print(version, len(rules), 'registered rules')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('versions', nargs='+')
    parser.add_argument('--cache', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, default=pathlib.Path('crates/msc-infrastructure/data/gamerules'))
    args = parser.parse_args()
    manifest = json.load(urllib.request.urlopen(MANIFEST))
    for version in args.versions:
        extract(version, manifest, args.cache, args.output)
