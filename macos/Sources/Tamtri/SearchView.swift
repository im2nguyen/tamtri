import SwiftUI

struct SearchView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.dismiss) private var dismiss
    @FocusState private var queryFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Search")
                    .font(.title2.bold())
                Spacer()
                Button("Done") { dismiss() }
            }

            TextField("Search conversations", text: $store.searchQuery)
                .textFieldStyle(.roundedBorder)
                .focused($queryFocused)
                .onSubmit { store.runSearch() }
                .onChange(of: store.searchQuery) { _, _ in
                    store.runSearch()
                }

            Text(store.searchScopeMessage)
                .font(.caption)
                .foregroundStyle(.secondary)

            if store.searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                Text("Type to search titles, assistant text, and thinking blocks.")
                    .foregroundStyle(.secondary)
            } else if store.searchResults.isEmpty {
                Text("No matches. \(store.searchScopeMessage)")
                    .foregroundStyle(.secondary)
            } else {
                List(store.searchResults) { hit in
                    Button {
                        store.selectSearchHit(hit)
                        dismiss()
                    } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(hit.title)
                                .font(.headline)
                            Text(hit.snippet)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .lineLimit(2)
                            Text(hit.matchField.capitalized)
                                .font(.caption2)
                                .foregroundStyle(.tertiary)
                        }
                    }
                    .buttonStyle(.tamtriPlain)
                }
            }
        }
        .padding()
        .frame(minWidth: 460, minHeight: 360)
        .onAppear {
            queryFocused = true
            if store.searchScopeMessage.isEmpty {
                Task { await store.loadSearchScopeMessage() }
            }
        }
    }
}
