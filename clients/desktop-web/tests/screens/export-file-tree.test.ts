import { expect, it } from 'vitest';
import {
  exportFileTree,
  type ExportNode,
} from '../../src/lib/sections/components/export-file-tree';
// Essential: folder grouping must not drop or duplicate exported paths. Pure
// data, no rendering/filesystem/timers, expected <5 ms; not authorized to run.
it('preserves every file under its nested folder and counts descendants', () => {
  const files = [
    { path: 'mods/z.jar', byteSize: 3 },
    { path: 'config/quests/chapters/start.json', byteSize: 2 },
    { path: 'config/common.toml', byteSize: 1 },
    { path: 'mods/a.jar', byteSize: 4 },
  ];
  const tree = exportFileTree(files);
  expect(tree.map((node) => [node.name, node.count])).toEqual([
    ['config', 2],
    ['mods', 2],
  ]);
  const paths: string[] = [];
  function visit(nodes: ExportNode[]): void {
    for (const node of nodes) {
      if (node.children) visit(node.children);
      else paths.push(node.file!.path);
    }
  }
  visit(tree);
  expect(paths.sort()).toEqual(files.map((file) => file.path).sort());
  expect(tree[0].children?.[0].children?.[0].children?.[0].file?.path).toBe(
    'config/quests/chapters/start.json',
  );
});
