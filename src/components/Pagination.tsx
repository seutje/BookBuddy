export function Pagination({
  label,
  page,
}: {
  label: string;
  page: {
    offset: number;
    index: number;
    total: number;
    size: number;
    setIndex: (index: number) => void;
  };
}) {
  if (page.total <= page.size) return null;
  return (
    <nav className="actions pagination" aria-label={`${label} pages`}>
      <button
        className="quiet"
        disabled={page.index === 0}
        onClick={() => page.setIndex(page.index - 1)}
      >
        Previous {label}
      </button>
      <p role="status">
        Showing {page.offset + 1}–
        {Math.min(page.offset + page.size, page.total)} of {page.total}
      </p>
      <button
        className="quiet"
        disabled={page.offset + page.size >= page.total}
        onClick={() => page.setIndex(page.index + 1)}
      >
        Next {label}
      </button>
    </nav>
  );
}
