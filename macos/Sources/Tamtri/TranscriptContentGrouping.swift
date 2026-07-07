import Foundation

struct TranscriptBlockGroup: Identifiable {
    let id: Int
    let toolBlock: TranscriptContentBlock?
    let nested: [TranscriptContentBlock]
    let standalone: TranscriptContentBlock?

    init(
        id: Int,
        toolBlock: TranscriptContentBlock? = nil,
        nested: [TranscriptContentBlock] = [],
        standalone: TranscriptContentBlock? = nil
    ) {
        self.id = id
        self.toolBlock = toolBlock
        self.nested = nested
        self.standalone = standalone
    }
}

enum TranscriptContentGrouping {
    private static let nestableTypes: Set<String> = [
        "elicitation_request",
        "elicitation_response",
        "app_resource",
        "task_ref",
        "artifact",
        "tool_result",
    ]

    static func build(from blocks: [TranscriptContentBlock]) -> [TranscriptBlockGroup] {
        var groups: [TranscriptBlockGroup] = []
        var index = 0
        var groupId = 0
        while index < blocks.count {
            let block = blocks[index]
            if block.type == "tool_call", let callId = block.callId {
                var nested: [TranscriptContentBlock] = []
                var next = index + 1
                while next < blocks.count {
                    let candidate = blocks[next]
                    guard nestableTypes.contains(candidate.type) else {
                        break
                    }
                    let nestsUnderCall: Bool = {
                        if candidate.type == "tool_result" {
                            return candidate.callId == callId
                        }
                        return candidate.originToolCallId == callId
                    }()
                    guard nestsUnderCall else {
                        break
                    }
                    nested.append(candidate)
                    next += 1
                }
                groups.append(TranscriptBlockGroup(id: groupId, toolBlock: block, nested: nested))
                groupId += 1
                index = next
                continue
            }
            groups.append(TranscriptBlockGroup(id: groupId, standalone: block))
            groupId += 1
            index += 1
        }
        return groups
    }
}

struct TranscriptArtifactRecord: Identifiable, Equatable, Hashable {
    var id: String { "\(path)|\(sha256)" }
    let path: String
    let size: UInt64
    let sha256: String
    let mimeType: String?
}

enum TranscriptArtifacts {
    static func extract(from messages: [ParsedTranscriptMessage]) -> [TranscriptArtifactRecord] {
        var seen = Set<String>()
        var artifacts: [TranscriptArtifactRecord] = []
        for message in messages {
            for block in message.content where block.type == "artifact" {
                guard let path = block.path,
                      let size = block.size,
                      let sha256 = block.sha256
                else {
                    continue
                }
                let key = "\(path)|\(sha256)"
                guard seen.insert(key).inserted else { continue }
                artifacts.append(
                    TranscriptArtifactRecord(
                        path: path,
                        size: size,
                        sha256: sha256,
                        mimeType: block.mimeType
                    )
                )
            }
        }
        return artifacts
    }
}
