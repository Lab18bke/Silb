import { BOOKMARKS_BAR, OTHER_BOOKMARKS, type Bookmark } from './types';

export interface FolderOption {
  id: number;
  title: string;
  depth: number;
}

export function childrenOf(all: Bookmark[], parent: number): Bookmark[] {
  return all.filter((b) => b.parentId === parent).sort((a, b) => a.position - b.position);
}

export function folderOptions(all: Bookmark[], exclude?: number): FolderOption[] {
  const out: FolderOption[] = [];
  const walk = (parent: number, depth: number) => {
    for (const b of childrenOf(all, parent)) {
      if (!b.isFolder || b.id === exclude) continue;
      out.push({ id: b.id, title: b.title, depth });
      walk(b.id, depth + 1);
    }
  };
  for (const root of [BOOKMARKS_BAR, OTHER_BOOKMARKS]) {
    const r = all.find((b) => b.id === root);
    if (r) out.push({ id: r.id, title: r.title, depth: 0 });
    walk(root, 1);
  }
  return out;
}
