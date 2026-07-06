import React from "react";
import ReactMarkdown from "react-markdown";
import type { TranscriptMessage } from "./main";

type Props = {
  messages: TranscriptMessage[];
};

export function TranscriptApp({ messages }: Props) {
  return (
    <div className="transcript" role="log" aria-live="polite">
      {messages.map((message) => (
        <article
          key={message.id}
          className={`message message--${message.role}`}
          tabIndex={0}
          aria-label={`${message.role} message`}
        >
          <header className="message__header">
            <span className="message__role">{message.role}</span>
            {message.harness_id ? (
              <span className="message__harness">{message.harness_id}</span>
            ) : null}
          </header>
          <div className="message__blocks">
            {message.content.map((block, index) => (
              <BlockCard key={`${message.id}-${index}`} block={block} />
            ))}
          </div>
        </article>
      ))}
    </div>
  );
}

function BlockCard({ block }: { block: TranscriptMessage["content"][number] }) {
  switch (block.type) {
    case "text":
      return (
        <section className="card card--text" tabIndex={0} aria-label="Text">
          <ReactMarkdown>{block.text ?? ""}</ReactMarkdown>
        </section>
      );
    case "thinking":
      return (
        <details className="card card--thinking" aria-label="Thinking">
          <summary>Thinking</summary>
          <ReactMarkdown>{block.text ?? ""}</ReactMarkdown>
        </details>
      );
    case "tool_call":
      return (
        <section
          className="card card--tool"
          tabIndex={0}
          aria-label={`Tool call ${block.name ?? "unknown"}`}
        >
          <strong>{block.name ?? "Tool"}</strong>
          {block.call_id ? <div className="muted">{block.call_id}</div> : null}
        </section>
      );
    case "tool_result":
      return (
        <section className="card card--tool-result" tabIndex={0} aria-label="Tool result">
          <strong>Result</strong>
          <pre>{block.text ?? ""}</pre>
        </section>
      );
    default:
      return null;
  }
}
