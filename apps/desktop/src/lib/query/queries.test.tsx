import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { makeTestQueryClient } from "../test/renderWithClient";

const { listRepos, setIssueState } = vi.hoisted(() => ({
  listRepos: vi.fn(),
  setIssueState: vi.fn(),
}));
vi.mock("../api", () => ({
  api: {
    listRepos: (...a: unknown[]) => listRepos(...a),
    setIssueState: (...a: unknown[]) => setIssueState(...a),
  },
}));

import { useRepos, useSetIssueState, queryKeys } from "./queries";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function wrapperWith() {
  const client = makeTestQueryClient();
  function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  }
  return { client, Wrapper };
}

describe("useRepos", () => {
  beforeEach(() => listRepos.mockReset());

  it("returns data from the api and caches under the account-scoped key", async () => {
    const repos = [{ id: "r-1", full_name: "acme/widget" }];
    listRepos.mockResolvedValue(repos);
    const { client, Wrapper } = wrapperWith();

    const { result } = renderHook(() => useRepos("acc-1"), {
      wrapper: Wrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual(repos);
    expect(listRepos).toHaveBeenCalledWith("acc-1");
    // The data is cached under ['repos', 'acc-1'].
    expect(client.getQueryData(queryKeys.repos("acc-1"))).toEqual(repos);
  });

  it("does not fire without an account id", async () => {
    const { Wrapper } = wrapperWith();
    const { result } = renderHook(() => useRepos(null), { wrapper: Wrapper });
    expect(result.current.fetchStatus).toBe("idle");
    expect(listRepos).not.toHaveBeenCalled();
  });
});

describe("useSetIssueState", () => {
  beforeEach(() => setIssueState.mockReset().mockResolvedValue(undefined));

  it("invalidates the account's issues + pulls queries on success", async () => {
    const { client, Wrapper } = wrapperWith();
    const invalidate = vi.spyOn(client, "invalidateQueries");

    const { result } = renderHook(() => useSetIssueState("acc-1"), {
      wrapper: Wrapper,
    });

    result.current.mutate({
      id: "acc-1",
      owner: "acme",
      repo: "widget",
      number: 1,
      state: "closed",
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(setIssueState).toHaveBeenCalledTimes(1);
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: queryKeys.issues("acc-1"),
    });
    expect(invalidate).toHaveBeenCalledWith({
      queryKey: queryKeys.pulls("acc-1"),
    });
  });
});
