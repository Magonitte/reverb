import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ToastHost } from "./Toast";
import { useUiStore } from "@/stores/ui";
import "@/lib/i18n";
beforeEach(() => {
  vi.useFakeTimers();
  useUiStore.setState({ toasts: [] });
});
afterEach(() => {
  vi.useRealTimers();
});
it("pauses countdown for pointer and keyboard and never acts on timeout", () => {
  const onAction = vi.fn();
  render(<ToastHost />);
  act(() => {
    useUiStore
      .getState()
      .pushToast({
        message: "Decida",
        actionLabel: "Baixar",
        onAction,
        secondaryActionLabel: "Cancelar",
        durationMs: 35000,
      });
  });
  const toast = screen.getByTestId("toast");
  act(() => {
    vi.advanceTimersByTime(10000);
  });
  expect(screen.getByTestId("toast-timer").style.transform).toBe(`scaleX(${25000 / 35000})`);
  fireEvent.mouseEnter(toast);
  act(() => {
    vi.advanceTimersByTime(60000);
  });
  expect(screen.getByText("Decida")).toBeVisible();
  fireEvent.mouseLeave(toast);
  fireEvent.focus(screen.getByRole("button", { name: "Baixar" }));
  act(() => {
    vi.advanceTimersByTime(60000);
  });
  expect(screen.getByText("Decida")).toBeVisible();
  fireEvent.blur(screen.getByRole("button", { name: "Baixar" }));
  act(() => {
    vi.advanceTimersByTime(25000);
  });
  expect(screen.queryByText("Decida")).not.toBeInTheDocument();
  expect(onAction).not.toHaveBeenCalled();
});
it("cancel closes without starting the download", () => {
  const onAction = vi.fn();
  render(<ToastHost />);
  act(() => {
    useUiStore
      .getState()
      .pushToast({
        message: "Link",
        actionLabel: "Baixar",
        onAction,
        secondaryActionLabel: "Cancelar",
      });
  });
  fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));
  expect(onAction).not.toHaveBeenCalled();
  expect(screen.queryByText("Link")).not.toBeInTheDocument();
});
it("prevents duplicate confirmations while an action is pending", async () => {
  let resolve!: () => void;
  const onAction = vi.fn(
    () =>
      new Promise<void>((r) => {
        resolve = r;
      }),
  );
  render(<ToastHost />);
  act(() => {
    useUiStore.getState().pushToast({ message: "Link", actionLabel: "Baixar", onAction });
  });
  const button = screen.getByRole("button", { name: "Baixar" });
  fireEvent.click(button);
  fireEvent.click(button);
  expect(onAction).toHaveBeenCalledTimes(1);
  expect(button).toBeDisabled();
  await act(async () => {
    resolve();
  });
  expect(screen.queryByText("Link")).not.toBeInTheDocument();
});
