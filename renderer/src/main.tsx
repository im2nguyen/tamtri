import React from "react";
import { createRoot } from "react-dom/client";
import { TranscriptApp } from "./TranscriptApp";
import "./styles.css";

declare global {
  interface Window {
    tamtriRender?: (messagesJSON: string) => void;
    tamtriRenderBase64?: (payload: string) => void;
    tamtriReportHeight?: () => number;
  }
}

const root = createRoot(document.getElementById("root")!);

function reportHeight(): number {
  const el = document.documentElement;
  return Math.max(el.scrollHeight, el.offsetHeight, 48);
}

window.tamtriReportHeight = reportHeight;

function render(messagesJSON: string) {
  try {
    const messages = JSON.parse(messagesJSON) as TranscriptMessage[];
    root.render(<TranscriptApp messages={messages} />);
    requestAnimationFrame(() => {
      const height = reportHeight();
      const handler = (window as unknown as { webkit?: { messageHandlers?: { tamtriHeight?: { postMessage: (v: number) => void } } } }).webkit?.messageHandlers?.tamtriHeight;
      handler?.postMessage(height);
    });
  } catch {
    root.render(<div role="alert">Unable to render transcript payload.</div>);
  }
}

window.tamtriRender = render;
window.tamtriRenderBase64 = (payload: string) => {
  render(atob(payload));
};
render("[]");

export type TranscriptBlock = {
  type: string;
  text?: string | null;
  name?: string | null;
  call_id?: string | null;
  input?: string | null;
  status?: string | null;
};

export type TranscriptMessage = {
  id: string;
  role: string;
  harness_id?: string | null;
  content: TranscriptBlock[];
};
