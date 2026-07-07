import React, { useCallback, useState } from "react";
import ReactMarkdown from "react-markdown";
import type { TranscriptMessage } from "./main";
import { normalizedTranscriptMarkdown } from "./transcriptMarkdown";

type Props = {
  messages: TranscriptMessage[];
};

type ContentBlock = TranscriptMessage["content"][number];

type Segment =
  | { kind: "text"; block: ContentBlock }
  | { kind: "activity"; blocks: ContentBlock[] };

export function TranscriptApp({ messages }: Props) {
  return (
    <div className="transcript" role="log" aria-live="polite">
      {messages.map((message) => (
        <article
          key={message.id}
          className={`message message--${message.role}`}
          aria-label={message.role === "user" ? "You" : "Assistant"}
        >
          <div className="message__blocks">
            {segmentBlocks(message.content).map((segment, index) =>
              segment.kind === "text" ? (
                <TextBlock key={`${message.id}-text-${index}`} block={segment.block} />
              ) : (
                <ActivityCluster
                  key={`${message.id}-activity-${index}`}
                  blocks={segment.blocks}
                  clusterKey={`${message.id}-activity-${index}`}
                />
              ),
            )}
          </div>
        </article>
      ))}
    </div>
  );
}

function segmentBlocks(content: ContentBlock[]): Segment[] {
  const segments: Segment[] = [];
  let activityRun: ContentBlock[] = [];

  const flushActivity = () => {
    if (activityRun.length === 0) return;
    segments.push({ kind: "activity", blocks: activityRun });
    activityRun = [];
  };

  for (const block of content) {
    if (block.type === "thinking" || block.type === "tool_call" || block.type === "tool_result") {
      if (block.type === "tool_call" && isEmptyToolCall(block)) {
        continue;
      }
      if (block.type === "tool_result") {
        const callId = block.call_id ?? null;
        const pendingCallIndex = callId
          ? activityRun.findIndex(
              (candidate) => candidate.type === "tool_call" && candidate.call_id === callId,
            )
          : -1;
        if (pendingCallIndex >= 0) {
          activityRun.splice(pendingCallIndex, 1);
        }
      }
      activityRun.push(block);
    } else if (block.type === "text") {
      flushActivity();
      segments.push({ kind: "text", block });
    } else {
      flushActivity();
    }
  }
  flushActivity();
  return segments;
}

function TextBlock({ block }: { block: ContentBlock }) {
  const markdown = normalizedTranscriptMarkdown(block.text ?? "");
  return (
    <section className="card card--text" aria-label="Text">
      <ReactMarkdown>{markdown}</ReactMarkdown>
    </section>
  );
}

function ActivityCluster({ blocks, clusterKey }: { blocks: ContentBlock[]; clusterKey: string }) {
  const [open, setOpen] = useState(false);
  const summary = summarizeActivity(blocks);

  const toggle = useCallback(() => {
    setOpen((current) => {
      reportHeightSoon();
      return !current;
    });
  }, []);

  if (!summary) return null;

  if (blocks.length === 1) {
    return (
      <section className="card card--muted card--activity">
        <ActivityItem block={blocks[0]} />
      </section>
    );
  }

  return (
    <section className="card card--muted card--activity" aria-label={`Activity: ${summary}`}>
      <button type="button" className="card__muted-toggle" onClick={toggle} aria-expanded={open}>
        <span className="card__muted-label">{summary}</span>
        <span className="card__chevron" aria-hidden="true">
          {open ? "▾" : "▸"}
        </span>
      </button>
      {open ? (
        <div className="card__activity-list">
          {blocks.map((block, index) => (
            <ActivityItem key={`${clusterKey}-${index}`} block={block} />
          ))}
        </div>
      ) : null}
    </section>
  );
}

function ActivityItem({ block }: { block: ContentBlock }) {
  const [open, setOpen] = useState(false);

  if (block.type === "thinking") {
    const text = (block.text ?? "").trim();
    if (!text) return null;
    return (
      <div className="card__activity-item">
        <button
          type="button"
          className="card__activity-line"
          onClick={() => {
            setOpen((current) => !current);
            reportHeightSoon();
          }}
          aria-expanded={open}
        >
          <span className="card__activity-kind">Thought</span>
          {text.length > 48 ? null : <span className="card__activity-detail">{text}</span>}
          <span className="card__chevron" aria-hidden="true">
            {open ? "▾" : "▸"}
          </span>
        </button>
        {open ? (
          <div className="card__muted-body card__markdown">
            <ReactMarkdown>{normalizedTranscriptMarkdown(text)}</ReactMarkdown>
          </div>
        ) : null}
      </div>
    );
  }

  if (block.type === "tool_call") {
    return <ToolActivityItem block={block} />;
  }

  if (block.type === "tool_result") {
    return <ToolResultActivityItem block={block} />;
  }

  return null;
}

function ToolActivityItem({ block }: { block: ContentBlock }) {
  const [open, setOpen] = useState(false);
  const fields = parseToolFields(block.input);
  const title = (block.name ?? "").trim() || "Tool";
  const preview = fields.map(([, value]) => value).join(" · ");
  const summary = toolSummaryLine(title, preview, block.status ?? null);
  const hasDetail = fields.length > 0;

  if (!hasDetail) {
    return (
      <div className="card__activity-item">
        <span className="card__activity-line card__activity-line--static">
          <span className="card__activity-kind">{title}</span>
          {preview ? <span className="card__activity-detail">{preview}</span> : null}
        </span>
      </div>
    );
  }

  return (
    <div className="card__activity-item">
      <button
        type="button"
        className="card__activity-line"
        onClick={() => {
          setOpen((current) => !current);
          reportHeightSoon();
        }}
        aria-expanded={open}
      >
        <span className="card__activity-kind">{title}</span>
        <span className="card__activity-detail">{summary.replace(`${title} · `, "")}</span>
        <span className="card__chevron" aria-hidden="true">
          {open ? "▾" : "▸"}
        </span>
      </button>
      {open ? (
        <div className="card__muted-body">
          <dl className="card__field">
            {fields.flatMap(([label, value]) => [
              <dt key={`${label}-l`} className="card__field-label">
                {label}
              </dt>,
              <dd key={`${label}-v`} className="card__field-value">
                {value}
              </dd>,
            ])}
          </dl>
        </div>
      ) : null}
    </div>
  );
}

function ToolResultActivityItem({ block }: { block: ContentBlock }) {
  const [open, setOpen] = useState(false);
  const text = (block.text ?? "").trim();
  const kind = (block.name ?? "").trim() || "Tool result";
  const status = (block.status ?? "").trim();
  const title =
    kind && status ? `${kind} ${status.replaceAll("_", " ")}` : kind || "Tool result";
  const hasDetail = text.length > 0;

  return (
    <div className="card__activity-item">
      <button
        type="button"
        className="card__activity-line"
        onClick={() => {
          if (!hasDetail) return;
          setOpen((current) => !current);
          reportHeightSoon();
        }}
        aria-expanded={open}
        disabled={!hasDetail}
      >
        <span className="card__activity-kind">{title}</span>
        {hasDetail ? (
          <span className="card__chevron" aria-hidden="true">
            {open ? "▾" : "▸"}
          </span>
        ) : null}
      </button>
      {open && hasDetail ? (
        <div className="card__muted-body">
          <pre className="card__muted-pre">{text}</pre>
        </div>
      ) : null}
    </div>
  );
}

function summarizeActivity(blocks: ContentBlock[]): string {
  let thoughts = 0;
  let reads = 0;
  let writes = 0;
  let searches = 0;
  let executes = 0;
  let other = 0;

  for (const block of blocks) {
    if (block.type === "thinking") {
      thoughts += 1;
      continue;
    }
    const name = (block.name ?? "").toLowerCase();
    if (name.includes("read")) reads += 1;
    else if (name.includes("write") || name.includes("edit")) writes += 1;
    else if (name.includes("search") || name.includes("grep")) searches += 1;
    else if (name.includes("execute") || name.includes("bash") || name.includes("command")) executes += 1;
    else other += 1;
  }

  const parts: string[] = [];
  if (thoughts) parts.push(thoughts === 1 ? "1 thought" : `${thoughts} thoughts`);
  if (reads) parts.push(reads === 1 ? "1 read" : `${reads} reads`);
  if (writes) parts.push(writes === 1 ? "1 edit" : `${writes} edits`);
  if (searches) parts.push(searches === 1 ? "1 search" : `${searches} searches`);
  if (executes) parts.push(executes === 1 ? "1 command" : `${executes} commands`);
  if (other) parts.push(other === 1 ? "1 tool" : `${other} tools`);
  return parts.join(", ");
}

function toolSummaryLine(title: string, subtitle: string, status?: string | null): string {
  const trimmedTitle = title.trim();
  const trimmedSubtitle = subtitle.trim();
  if (!trimmedSubtitle) return trimmedTitle;
  const statusWords = new Set(["started", "completed", "failed", "pending", "in_progress", "in progress"]);
  if (status && statusWords.has(status.toLowerCase())) {
    return trimmedSubtitle.length > trimmedTitle.length ? trimmedSubtitle : trimmedTitle;
  }
  if (statusWords.has(trimmedSubtitle.toLowerCase())) {
    return trimmedTitle;
  }
  return `${trimmedTitle} · ${trimmedSubtitle}`;
}

function isEmptyToolCall(block: ContentBlock) {
  const name = (block.name ?? "").trim();
  const input = (block.input ?? "").trim();
  return !name && !input;
}

function parseToolFields(input: string | null | undefined): [string, string][] {
  if (!input) return [];
  try {
    const parsed = JSON.parse(input) as Record<string, unknown>;
    const priority = ["command", "path", "file_path", "query", "pattern", "description"];
    const entries: [string, string][] = [];
    for (const key of priority) {
      if (parsed[key] != null) {
        entries.push([key.replace(/_/g, " "), String(parsed[key])]);
      }
    }
    if (entries.length < 4) {
      for (const [key, value] of Object.entries(parsed)) {
        if (entries.some(([k]) => k === key.replace(/_/g, " "))) continue;
        if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
          entries.push([key.replace(/_/g, " "), String(value)]);
        }
        if (entries.length >= 4) break;
      }
    }
    return entries.slice(0, 4);
  } catch {
    return input.trim() ? [["input", input.slice(0, 200)]] : [];
  }
}

function reportHeightSoon() {
  requestAnimationFrame(() => {
    const height = window.tamtriReportHeight?.();
    if (height == null) return;
    const handler = (
      window as unknown as {
        webkit?: { messageHandlers?: { tamtriHeight?: { postMessage: (v: number) => void } } };
      }
    ).webkit?.messageHandlers?.tamtriHeight;
    handler?.postMessage(height);
  });
}
