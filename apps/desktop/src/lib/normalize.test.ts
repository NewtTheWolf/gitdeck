import { describe, expect, it, vi } from "vitest";

// api.ts imports transport (which pulls in the generated client). Mock the
// transport so importing api.ts in a unit test doesn't construct a real client.
vi.mock("./transport", () => ({
  getGrpcClient: vi.fn(),
  isRemote: vi.fn(() => false),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { protoToSnake, camelToSnake } from "./api";

describe("camelToSnake", () => {
  it("converts camelCase to snake_case", () => {
    expect(camelToSnake("repoNameWithOwner")).toBe("repo_name_with_owner");
    expect(camelToSnake("isDraft")).toBe("is_draft");
    expect(camelToSnake("htmlUrl")).toBe("html_url");
  });
  it("leaves already-snake keys unchanged", () => {
    expect(camelToSnake("already_snake")).toBe("already_snake");
    expect(camelToSnake("id")).toBe("id");
  });
  it("handles digit boundaries", () => {
    expect(camelToSnake("authorAvatarUrl2")).toBe("author_avatar_url2");
  });
});

describe("protoToSnake", () => {
  it("deep-converts nested object keys", () => {
    const input = {
      repoNameWithOwner: "acme/widget",
      authorLogin: "octocat",
      nested: { someValue: 1, deepNest: { aB: 2 } },
    };
    expect(protoToSnake(input)).toEqual({
      repo_name_with_owner: "acme/widget",
      author_login: "octocat",
      nested: { some_value: 1, deep_nest: { a_b: 2 } },
    });
  });

  it("converts keys inside arrays element-wise", () => {
    const input = {
      labels: [
        { name: "bug", colorHex: "f00" },
        { name: "wip", colorHex: "0f0" },
      ],
    };
    expect(protoToSnake(input)).toEqual({
      labels: [
        { name: "bug", color_hex: "f00" },
        { name: "wip", color_hex: "0f0" },
      ],
    });
  });

  it("passes primitives through untouched", () => {
    expect(protoToSnake(42)).toBe(42);
    expect(protoToSnake("hello")).toBe("hello");
    expect(protoToSnake(true)).toBe(true);
    expect(protoToSnake(null)).toBe(null);
  });

  it("converts arrays of primitives unchanged", () => {
    expect(protoToSnake(["a", "b", "c"])).toEqual(["a", "b", "c"]);
  });

  it("drops internal protobuf keys starting with $", () => {
    const input = {
      $typeName: "gitdeck.v1.Task",
      $unknown: undefined,
      sourceUrl: "https://x",
    };
    expect(protoToSnake(input)).toEqual({ source_url: "https://x" });
  });
});
