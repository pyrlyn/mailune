// Mounts a component into happy-dom with React's act, so a test can click
// and read the result without a browser or a testing library.

import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";

declare global {
  var IS_REACT_ACT_ENVIRONMENT: boolean | undefined;
}

globalThis.IS_REACT_ACT_ENVIRONMENT = true;

export async function mount(node: ReactNode): Promise<{ container: HTMLElement; unmount: () => Promise<void> }> {
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  await act(async () => root.render(node));
  return {
    container,
    unmount: async () => {
      await act(async () => root.unmount());
      container.remove();
    },
  };
}

export async function click(element: Element | null | undefined): Promise<void> {
  if (!(element instanceof HTMLElement)) {
    throw new Error("nothing to click");
  }
  await act(async () => element.click());
}

/** The pane whose accessible name is `label`. */
export function pane(container: HTMLElement, label: string): HTMLElement {
  const found = container.querySelector<HTMLElement>(`[aria-label="${label}"]`);
  if (!found) {
    throw new Error(`no pane named ${label}`);
  }
  return found;
}
