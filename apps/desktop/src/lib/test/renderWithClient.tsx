import type { ReactElement, ReactNode } from "react";
import { render, type RenderOptions } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

/**
 * A fresh QueryClient with retries disabled — tests should fail fast and never
 * wait on the default 1-retry backoff. gcTime is short so caches don't leak
 * between tests.
 */
export function makeTestQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  });
}

/**
 * Render a component wrapped in a fresh QueryClientProvider. Components that
 * also need a router should pass the router-wrapped element as `ui` (the view
 * tests already wrap in MemoryRouter where needed).
 */
export function renderWithClient(
  ui: ReactElement,
  options?: { client?: QueryClient } & Omit<RenderOptions, "wrapper">,
) {
  const client = options?.client ?? makeTestQueryClient();
  function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  }
  return {
    client,
    ...render(ui, { wrapper: Wrapper, ...options }),
  };
}
