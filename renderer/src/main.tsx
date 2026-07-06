import React from "react";
import { createRoot } from "react-dom/client";
import { TranscriptApp } from "./TranscriptApp";
import "./styles.css";

declare global {
  interface Window {
    tamtriRender?: (messagesJSON: string) => void;
  }
}

const root = createRoot(document.getElementById("root")!);

function render(messagesJSON: string) {
  try {
    const messages = JSON.parse(messagesJSON) as TranscriptMessage[];
    root.render(<TranscriptApp messages={messages} />);
  } catch {
    root.render(<div role="alert">Unable to render transcript payload.</div>);
  }
}

window.tamtriRender = render;
render("[]");

export type TranscriptBlock = {
  type: string;
  text?: string | null;
  name?: string | null;
  call_id?: string | null;
};

export type TranscriptMessage = {
  id: string;
  role: string;
  harness_id?: string | null;
  content: TranscriptBlock[];
};
