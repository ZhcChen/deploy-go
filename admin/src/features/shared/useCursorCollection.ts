import { useInfiniteQuery } from "@tanstack/react-query";

export interface CursorPage<T> {
  items: T[];
  nextCursor?: string | null;
}

export function useCursorCollection<T>(
  queryKey: readonly unknown[],
  load: (after: string | null) => Promise<CursorPage<T>>,
  refresh?: { intervalMs: number },
) {
  const query = useInfiniteQuery({
    queryKey,
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) => load(pageParam),
    getNextPageParam: (page) => page.nextCursor ?? undefined,
    refetchInterval: refresh?.intervalMs,
    refetchIntervalInBackground: false,
    refetchOnWindowFocus: Boolean(refresh),
  });
  const seen = new Set<string>();
  const items = query.data?.pages.flatMap((page) => page.items).filter((item) => {
    const id = (item as { id?: string; applicationId?: string }).id ?? (item as { applicationId?: string }).applicationId;
    if (!id || seen.has(id)) return false;
    seen.add(id);
    return true;
  }) ?? [];
  return { ...query, items };
}
