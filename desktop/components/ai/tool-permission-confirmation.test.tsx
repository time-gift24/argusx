import userEvent from "@testing-library/user-event";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ToolPermissionConfirmation } from "@/components/ai/tool-permission-confirmation";

describe("ToolPermissionConfirmation", () => {
  it("renders request and resolved states with the expected actions", async () => {
    const user = userEvent.setup();
    const onAllow = vi.fn();
    const onDeny = vi.fn();
    const { rerender } = render(
      <ToolPermissionConfirmation
        argumentsSummary='{"command":"git status"}'
        onAllow={onAllow}
        onDeny={onDeny}
        requestId="perm-1"
        state="requested"
        toolName="shell"
      />
    );

    expect(screen.getByText("Tool permission required.")).toBeInTheDocument();
    expect(screen.getByText("shell")).toBeInTheDocument();
    expect(screen.getByText('{"command":"git status"}')).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Allow" }));
    await user.click(screen.getByRole("button", { name: "Deny" }));
    expect(onAllow).toHaveBeenCalledTimes(1);
    expect(onDeny).toHaveBeenCalledTimes(1);

    rerender(
      <ToolPermissionConfirmation
        requestId="perm-1"
        state="accepted"
        toolName="shell"
      />
    );
    expect(screen.getByText("Tool request approved.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Allow" })).not.toBeInTheDocument();

    rerender(
      <ToolPermissionConfirmation
        requestId="perm-1"
        state="rejected"
        toolName="shell"
      />
    );
    expect(screen.getByText("Tool request denied.")).toBeInTheDocument();
  });
});
