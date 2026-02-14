/**
 * Button Component Unit Tests
 */

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

describe("Button Component", () => {
  it("renders correctly", () => {
    render(<button>Click me</button>);
    expect(screen.getByText("Click me")).toBeInTheDocument();
  });

  it("handles click events", () => {
    const handleClick = vi.fn();
    render(<button onClick={handleClick}>Click me</button>);
    
    fireEvent.click(screen.getByText("Click me"));
    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it("is disabled when disabled prop is true", () => {
    render(<button disabled>Disabled</button>);
    expect(screen.getByText("Disabled")).toBeDisabled();
  });
});
