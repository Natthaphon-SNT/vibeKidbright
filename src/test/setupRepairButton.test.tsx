import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SetupRepairButton from "../SetupRepairButton";

describe("SetupRepairButton", () => {
  it("is disabled while the shared toolchain installation is active", () => {
    const onClick = vi.fn();
    render(<SetupRepairButton isInstalling={true} onClick={onClick} />);

    const button = screen.getByRole("button", { name: /Installing ESP-IDF/i });
    expect(button).toBeDisabled();
    fireEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("opens setup when no installation is active", () => {
    const onClick = vi.fn();
    render(<SetupRepairButton isInstalling={false} onClick={onClick} />);

    const button = screen.getByRole("button", { name: /Setup \/ Repair ESP-IDF/i });
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
  });
});
