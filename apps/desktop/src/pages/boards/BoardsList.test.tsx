import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";

const navigate = vi.fn();
vi.mock("react-router-dom", async (orig) => {
  const actual = await orig<typeof import("react-router-dom")>();
  return { ...actual, useNavigate: () => navigate };
});

const { listBoards, createBoard } = vi.hoisted(() => ({
  listBoards: vi.fn(),
  createBoard: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    listBoards: (...a: unknown[]) => listBoards(...a),
    createBoard: (...a: unknown[]) => createBoard(...a),
  },
}));

import "../../lib/i18n";
import BoardsList from "./BoardsList";

function renderList() {
  return renderWithClient(
    <MemoryRouter>
      <BoardsList />
    </MemoryRouter>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("BoardsList", () => {
  beforeEach(() => {
    navigate.mockReset();
    listBoards.mockReset();
    createBoard.mockReset();
  });

  it("renders boards returned by the api", async () => {
    listBoards.mockResolvedValue([
      {
        id: "b-1",
        name: "Sprint Board",
        position: 0,
        created_at: "2026-06-01T00:00:00Z",
        updated_at: "2026-06-15T00:00:00Z",
        columns: [],
      },
    ]);
    renderList();
    expect(await screen.findByText("Sprint Board")).toBeInTheDocument();
  });

  it("shows the empty state when there are no boards", async () => {
    listBoards.mockResolvedValue([]);
    renderList();
    expect(await screen.findByText(/No boards yet/i)).toBeInTheDocument();
  });

  it("creates a board and navigates into it", async () => {
    listBoards.mockResolvedValue([]);
    createBoard.mockResolvedValue({
      id: "b-new",
      name: "My Board",
      position: 0,
      created_at: "x",
      updated_at: "x",
      columns: [],
    });
    renderList();
    await screen.findByText(/No boards yet/i);

    fireEvent.click(screen.getByText("New board"));
    const input = screen.getByPlaceholderText("Board name");
    fireEvent.change(input, { target: { value: "My Board" } });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() =>
      expect(createBoard).toHaveBeenCalledWith("My Board"),
    );
    await waitFor(() =>
      expect(navigate).toHaveBeenCalledWith("/boards/b-new"),
    );
  });
});
