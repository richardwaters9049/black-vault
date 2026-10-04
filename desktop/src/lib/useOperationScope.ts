import { useCallback, useLayoutEffect, useRef } from 'react';

// Ignore late native responses after a view closes or a newer request replaces it.
export function useOperationScope(scope?: string) {
  const generation = useRef(0);
  const invalidate = useCallback(() => { generation.current += 1; }, []);
  useLayoutEffect(() => {
    invalidate();
    return invalidate;
  }, [scope, invalidate]);
  const start = useCallback(() => {
    const request = ++generation.current;
    return () => generation.current === request;
  }, []);
  return { start, invalidate };
}
