import Foundation
import WebKit

enum AppWebViewBridge {
    static let handlerName = "tamtriAppBridge"
}

struct AppBridgeContext: Equatable {
    let conversationId: String
    let serverId: String
    let appId: String
    let templateRef: String
    let bridgeScript: String
    let webViewID: UUID
}

struct AppBridgeSubmission: Equatable {
    let requestId: String
    let needsConsent: Bool
}

struct AppTemplateRecord: Equatable {
    let templateRef: String
    let serverId: String
    let html: String
    let allowedOrigins: [String]
    let bridgeScript: String
    let contentSecurityPolicy: String
}

struct AppBridgeConsentPayload: Decodable, Equatable {
    let requestId: String
    let serverId: String
    let appId: String
    let templateRef: String
    let summary: String
    let options: [AppBridgeConsentOption]

    enum CodingKeys: String, CodingKey {
        case requestId = "request_id"
        case serverId = "server_id"
        case appId = "app_id"
        case templateRef = "template_ref"
        case summary
        case options
    }
}

struct AppBridgeConsentOption: Decodable, Equatable, Identifiable {
    let id: String
    let label: String
}

struct BridgeDelivery: Equatable {
    let webViewID: UUID
    let responseJSON: String
}

struct LiveTaskState: Equatable {
    let taskId: String
    let serverId: String
    var status: String
    var title: String?
    var progressMessage: String?

    init(payloadJSON: String) {
        let json = JSONValue.from(json: payloadJSON)
        if let state = json?.value(at: "state") {
            taskId = state.string(at: "task_id") ?? state.string(at: "taskId") ?? "task"
            serverId = state.string(at: "server_id") ?? state.string(at: "serverId") ?? ""
            status = state.string(at: "status") ?? "running"
            title = state.string(at: "title")
            progressMessage = state.value(at: "progress")?.string(at: "message")
        } else {
            taskId = json?.string(at: "task_id") ?? json?.string(at: "taskId") ?? "task"
            serverId = json?.string(at: "server_id") ?? json?.string(at: "serverId") ?? ""
            status = json?.string(at: "status") ?? "running"
            title = json?.string(at: "title")
            progressMessage = json?.value(at: "progress")?.string(at: "message")
        }
    }

    var isTerminal: Bool {
        ["completed", "failed"].contains(status)
    }
}

@MainActor
protocol AppBridgeDelivering: AnyObject {
    func deliverBridgeResponse(webViewID: UUID, responseJSON: String)
}
