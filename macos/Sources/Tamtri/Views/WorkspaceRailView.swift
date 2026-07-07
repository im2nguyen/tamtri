import SwiftUI

struct WorkspaceRailView: View {
    @EnvironmentObject private var store: AppStore
    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        FilesPanelView()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(TamtriTheme.surfaceBase(colorScheme))
            .ignoresSafeArea(.container, edges: .top)
            .accessibilityLabel(store.workspaceRailMode == .preview ? "File preview panel" : "Files panel")
    }
}
