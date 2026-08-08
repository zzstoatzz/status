import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "./toast.svelte.ts";

describe("toast", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    toast.toasts = [];
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows newest first, so the container stacks in order", () => {
    toast.success("link copied");
    toast.success("status posted");

    expect(toast.toasts.map((t) => t.message)).toEqual(["status posted", "link copied"]);
  });

  it("auto-dismisses a success after its duration", () => {
    toast.success("link copied");
    expect(toast.toasts).toHaveLength(1);

    vi.advanceTimersByTime(2499);
    expect(toast.toasts).toHaveLength(1);

    vi.advanceTimersByTime(1);
    expect(toast.toasts).toHaveLength(0);
  });

  // a failure is worth reading, and may need acting on
  it("keeps an error up longer than a success", () => {
    toast.success("link copied");
    toast.error("could not copy link");

    vi.advanceTimersByTime(2500);
    expect(toast.toasts.map((t) => t.type)).toEqual(["error"]);

    vi.advanceTimersByTime(2500);
    expect(toast.toasts).toHaveLength(0);
  });

  it("dismisses only the one asked for", () => {
    const first = toast.success("one");
    toast.success("two");

    toast.dismiss(first);

    expect(toast.toasts.map((t) => t.message)).toEqual(["two"]);
  });

  it("gives each toast its own id, so repeating a message shows twice", () => {
    const a = toast.success("link copied");
    const b = toast.success("link copied");

    expect(a).not.toBe(b);
    expect(toast.toasts).toHaveLength(2);
  });
});
