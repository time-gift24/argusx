import { render, screen } from "@testing-library/react";

import { MessageResponse } from "./message";

describe("MessageResponse", () => {
  it("renders unordered markdown lists with non-hanging markers", () => {
    render(<MessageResponse>- list item</MessageResponse>);

    const list = screen.getByRole("list");
    expect(list.className).toContain("list-inside");
    expect(list.className).toContain("pl-0");
    expect(list.className).toContain("[li_&]:pl-6");
  });

  it("renders ordered markdown lists with non-hanging markers", () => {
    render(<MessageResponse>1. list item</MessageResponse>);

    const list = screen.getByRole("list");
    expect(list.className).toContain("list-inside");
    expect(list.className).toContain("pl-0");
    expect(list.className).toContain("[li_&]:pl-6");
  });

  it("keeps nested list indentation enabled", () => {
    render(
      <MessageResponse>{`- parent item\n  - child item`}</MessageResponse>
    );

    const lists = screen.getAllByRole("list");
    expect(lists).toHaveLength(2);
    expect(lists[1].className).toContain("[li_&]:pl-6");
    expect(lists[1].getAttribute("style") ?? "").not.toContain(
      "padding-inline-start: 0"
    );
  });
});
