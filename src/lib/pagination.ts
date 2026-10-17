import { useState } from "react";
export function usePage<T>(items: T[], key: string, size = 100) {
  const [position, setPosition] = useState({ key, index: 0 });
  const index = Math.min(
    position.key === key ? position.index : 0,
    Math.max(0, Math.ceil(items.length / size) - 1),
  );
  const offset = index * size;
  return {
    items: items.slice(offset, offset + size),
    offset,
    index,
    total: items.length,
    size,
    setIndex: (index: number) => setPosition({ key, index }),
  };
}
