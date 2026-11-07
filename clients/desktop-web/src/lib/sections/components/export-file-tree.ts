export interface ExportFile {
  path: string;
  byteSize: number;
  optional?: boolean;
  enabled?: boolean;
}
export interface ExportNode {
  name: string;
  path: string;
  children?: ExportNode[];
  file?: ExportFile;
  count: number;
}
export function exportFileTree(files: readonly ExportFile[]): ExportNode[] {
  const root: ExportNode[] = [];
  const folders = new Map<string, ExportNode>();
  for (const file of files) {
    const parts = file.path.split('/');
    let children = root;
    let prefix = '';
    for (const name of parts.slice(0, -1)) {
      prefix = prefix ? `${prefix}/${name}` : name;
      let folder = folders.get(prefix);
      if (!folder) {
        folder = { name, path: prefix, children: [], count: 0 };
        folders.set(prefix, folder);
        children.push(folder);
      }
      folder.count += 1;
      children = folder.children!;
    }
    children.push({ name: parts.at(-1)!, path: file.path, file, count: 1 });
  }
  function sort(nodes: ExportNode[]): void {
    nodes.sort(
      (a, b) =>
        Number(!!b.children) - Number(!!a.children) ||
        a.name.localeCompare(b.name, undefined, { numeric: true }),
    );
    for (const node of nodes) if (node.children) sort(node.children);
  }
  sort(root);
  return root;
}
