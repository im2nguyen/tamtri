import AppKit
import SwiftUI

struct HarnessHealthView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @State private var checklistText = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Text("Harness Health")
                    .font(.title2.bold())
                Spacer()
                Button {
                    Task { await store.refreshHarnessHealth() }
                } label: {
                    Label("Refresh", systemImage: "arrow.clockwise")
                }
                if store.hasReadyHarness {
                    Button("Done") {
                        dismiss()
                    }
                }
            }

            Text("tamtri detects installed ACP agents and guides setup. It does not install or manage harnesses for you.")
                .font(.callout)
                .foregroundStyle(.secondary)

            if store.harnessHealthEntries.isEmpty {
                Text("No agents configured yet. tamtri looks for Hermes, Claude Code, and Goose on your PATH and in common install locations. You can also add entries to ~/.tamtri/vault/config.json.")
                    .foregroundStyle(.secondary)
            } else {
                List(store.harnessHealthEntries) { entry in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack {
                            Text(entry.displayName)
                                .font(.headline)
                            Spacer()
                            HarnessHealthStatusBadge(status: entry.status)
                        }
                        Text(entry.command)
                            .font(.caption.monospaced())
                            .foregroundStyle(.secondary)
                        if !entry.installDocURL.isEmpty {
                            Link("Install documentation", destination: URL(string: entry.installDocURL)!)
                                .font(.caption)
                        }
                    }
                    .padding(.vertical, 4)
                }
                .frame(minHeight: 180)
            }

            GroupBox("IT / admin checklist") {
                ScrollView {
                    Text(checklistText.isEmpty ? "Loading checklist…" : checklistText)
                        .font(.caption.monospaced())
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .textSelection(.enabled)
                }
                .frame(minHeight: 120, maxHeight: 180)
                HStack {
                    Spacer()
                    Button("Copy checklist") {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(checklistText, forType: .string)
                    }
                    .disabled(checklistText.isEmpty)
                }
            }

            if !store.hasReadyHarness {
                Text("Install at least one ready harness to start a conversation.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        }
        .padding()
        .frame(minWidth: 520, minHeight: 420)
        .task {
            await store.refreshHarnessHealth()
            checklistText = await store.harnessHealthChecklist()
        }
    }
}

struct HarnessHealthStatusBadge: View {
    let status: String

    var body: some View {
        Text(label)
            .font(.caption.weight(.semibold))
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(color.opacity(0.18), in: Capsule())
            .foregroundStyle(color)
            .accessibilityLabel("Harness status \(label)")
    }

    private var label: String {
        switch status {
        case "ready": "Ready"
        case "missing": "Missing"
        default: "Unknown"
        }
    }

    private var color: Color {
        switch status {
        case "ready": .green
        case "missing": .orange
        default: .secondary
        }
    }
}
