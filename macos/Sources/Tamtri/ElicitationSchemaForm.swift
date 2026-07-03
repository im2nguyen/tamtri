import Foundation
import SwiftUI

enum ElicitationCardKind: Equatable {
    case urlHandoff
    case secretBlocked
    case unsupportedSchema
    case form
}

enum ElicitationCardRouter {
    static func cardKind(mode: String?, schema: JSONValue?) -> ElicitationCardKind {
        if mode == "url" {
            return .urlHandoff
        }
        if ElicitationSchemaPolicy.schemaLooksSecret(schema) {
            return .secretBlocked
        }
        if !ElicitationSchemaPolicy.schemaIsRenderable(schema) {
            return .unsupportedSchema
        }
        return .form
    }
}

enum ElicitationSchemaPolicy {
    static func textLooksSecret(_ text: String) -> Bool {
        let normalized = text.lowercased()
        let needles = [
            "password", "secret", "api_key", "api key", "apikey", "access_token",
            "access token", "refresh_token", "refresh token", "private_key",
            "private key", "bearer", "credential"
        ]
        return needles.contains { normalized.contains($0) }
    }

    static func fieldLooksSecret(name: String, descriptor: [String: JSONValue]) -> Bool {
        if textLooksSecret(name) {
            return true
        }
        for key in ["title", "description", "format"] {
            if let value = descriptor[key], case .string(let text) = value, textLooksSecret(text) {
                return true
            }
        }
        return false
    }

    static func schemaLooksSecret(_ schema: JSONValue?) -> Bool {
        guard case .object(let root) = schema,
              case .object(let properties) = root["properties"] ?? .null
        else {
            return false
        }
        return properties.contains { fieldLooksSecret(name: $0.key, descriptor: objectValue($0.value)) }
    }

    static func schemaIsRenderable(_ schema: JSONValue?) -> Bool {
        guard case .object(let root) = schema,
              case .object(let properties) = root["properties"] ?? .null
        else {
            return false
        }
        return properties.values.allSatisfy { propertyIsRenderable($0) }
    }

    private static func propertyIsRenderable(_ property: JSONValue) -> Bool {
        let object = objectValue(property)
        let type = stringValue(object["type"]) ?? "string"
        switch type {
        case "string", "number", "integer", "boolean":
            return true
        case "array":
            guard let items = object["items"] else { return true }
            return stringValue(objectValue(items)["type"]) != "object"
        case "object":
            return false
        default:
            return false
        }
    }

    private static func objectValue(_ value: JSONValue) -> [String: JSONValue] {
        if case .object(let object) = value {
            return object
        }
        return [:]
    }

    private static func stringValue(_ value: JSONValue?) -> String? {
        guard case .string(let text) = value else { return nil }
        return text
    }
}

struct ElicitationSchemaField: Identifiable, Equatable {
    let id: String
    let title: String
    let description: String?
    let type: String
    let itemType: String?
    let required: Bool
    let enumValues: [String]
    let minLength: Int?
    let maxLength: Int?
    let minimum: Double?
    let maximum: Double?
}

enum ElicitationSchemaParser {
    static func fields(from schema: JSONValue?) -> [ElicitationSchemaField] {
        guard case .object(let root) = schema,
              case .object(let properties) = root["properties"] ?? .null
        else {
            return []
        }
        let required = requiredFields(in: root)
        return properties.keys.sorted().compactMap { name in
            let descriptor = valueObject(properties[name])
            guard !ElicitationSchemaPolicy.fieldLooksSecret(name: name, descriptor: descriptor) else {
                return nil
            }
            let type = string(descriptor["type"]) ?? "string"
            let itemType: String?
            if type == "array" {
                let items = valueObject(descriptor["items"])
                itemType = string(items["type"]) ?? "string"
            } else {
                itemType = nil
            }
            let enumValues: [String]
            if case .array(let values) = descriptor["enum"] ?? .null {
                enumValues = values.compactMap { if case .string(let text) = $0 { text } else { nil } }
            } else {
                enumValues = []
            }
            return ElicitationSchemaField(
                id: name,
                title: string(descriptor["title"]) ?? name,
                description: string(descriptor["description"]),
                type: type,
                itemType: itemType,
                required: required.contains(name),
                enumValues: enumValues,
                minLength: int(descriptor["minLength"]),
                maxLength: int(descriptor["maxLength"]),
                minimum: number(descriptor["minimum"]),
                maximum: number(descriptor["maximum"])
            )
        }
    }

    private static func requiredFields(in root: [String: JSONValue]) -> Set<String> {
        guard case .array(let values) = root["required"] ?? .null else { return [] }
        return Set(values.compactMap { if case .string(let text) = $0 { text } else { nil } })
    }

    private static func valueObject(_ value: JSONValue?) -> [String: JSONValue] {
        if case .object(let object) = value { return object }
        return [:]
    }

    private static func string(_ value: JSONValue?) -> String? {
        if case .string(let text) = value { return text }
        return nil
    }

    private static func int(_ value: JSONValue?) -> Int? {
        if case .number(let number) = value { return Int(number) }
        return nil
    }

    private static func number(_ value: JSONValue?) -> Double? {
        if case .number(let number) = value { return number }
        return nil
    }
}

enum ElicitationSchemaFormBuilder {
    static func buildPayload(
        fields: [ElicitationSchemaField],
        values: [String: String],
        booleans: [String: Bool]
    ) -> (payload: [String: Any], error: String?)? {
        for field in fields {
            switch field.type {
            case "boolean":
                continue
            case "integer", "number":
                let raw = values[field.id, default: ""].trimmingCharacters(in: .whitespacesAndNewlines)
                if raw.isEmpty, field.required {
                    return nil
                }
                if !raw.isEmpty {
                    if field.type == "integer" {
                        guard let number = Int(raw) else {
                            return ([:], "\(field.title) must be a whole number.")
                        }
                        if let minimum = field.minimum, Double(number) < minimum {
                            return ([:], "\(field.title) must be at least \(formatBound(minimum)).")
                        }
                        if let maximum = field.maximum, Double(number) > maximum {
                            return ([:], "\(field.title) must be at most \(formatBound(maximum)).")
                        }
                    } else if let number = Double(raw) {
                        if let minimum = field.minimum, number < minimum {
                            return ([:], "\(field.title) must be at least \(formatBound(minimum)).")
                        }
                        if let maximum = field.maximum, number > maximum {
                            return ([:], "\(field.title) must be at most \(formatBound(maximum)).")
                        }
                    } else {
                        return ([:], "\(field.title) must be a number.")
                    }
                }
            case "array":
                let raw = values[field.id, default: ""]
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                if raw.isEmpty, field.required {
                    return nil
                }
            default:
                let raw = values[field.id, default: ""].trimmingCharacters(in: .whitespacesAndNewlines)
                if raw.isEmpty, field.required {
                    return nil
                }
                if let minLength = field.minLength, raw.count < minLength {
                    return ([:], "\(field.title) must be at least \(minLength) characters.")
                }
                if let maxLength = field.maxLength, raw.count > maxLength {
                    return ([:], "\(field.title) must be at most \(maxLength) characters.")
                }
            }
        }
        var payload: [String: Any] = [:]
        for field in fields {
            switch field.type {
            case "boolean":
                payload[field.id] = booleans[field.id, default: false]
            case "integer":
                if let raw = values[field.id], let number = Int(raw) { payload[field.id] = number }
            case "number":
                if let raw = values[field.id], let number = Double(raw) { payload[field.id] = number }
            case "array":
                guard let raw = values[field.id] else { continue }
                let parts = raw
                    .split(whereSeparator: { $0 == "," || $0 == "\n" })
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter { !$0.isEmpty }
                if parts.isEmpty { continue }
                switch field.itemType ?? "string" {
                case "integer":
                    let parsed = parts.compactMap(Int.init)
                    if !parsed.isEmpty { payload[field.id] = parsed }
                case "number":
                    let parsed = parts.compactMap(Double.init)
                    if !parsed.isEmpty { payload[field.id] = parsed }
                case "boolean":
                    let parsed = parts.compactMap { text -> Bool? in
                        switch text.lowercased() {
                        case "true", "1", "yes": return true
                        case "false", "0", "no": return false
                        default: return nil
                        }
                    }
                    if !parsed.isEmpty { payload[field.id] = parsed }
                default:
                    payload[field.id] = parts
                }
            default:
                if let raw = values[field.id], !raw.isEmpty { payload[field.id] = raw }
            }
        }
        return (payload, nil)
    }

    private static func formatBound(_ value: Double) -> String {
        if value.rounded() == value {
            return String(Int(value))
        }
        return String(value)
    }
}

struct ElicitationSchemaForm: View {
    let fields: [ElicitationSchemaField]
    @Binding var values: [String: String]
    @Binding var booleans: [String: Bool]
    @Binding var validationMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            ForEach(fields) { field in
                VStack(alignment: .leading, spacing: 4) {
                    Text(field.title + (field.required ? " *" : ""))
                        .font(.caption.bold())
                    if let description = field.description, !description.isEmpty {
                        Text(description)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    fieldView(field)
                }
            }
            if let validationMessage {
                Text(validationMessage)
                    .font(.caption)
                    .foregroundStyle(.red)
            }
        }
    }

    @ViewBuilder
    private func fieldView(_ field: ElicitationSchemaField) -> some View {
        switch field.type {
        case "boolean":
            Toggle(field.title, isOn: binding(for: field.id, default: false))
                .accessibilityLabel(Text(field.title))
        case "string" where !field.enumValues.isEmpty:
            Picker("", selection: binding(for: field.id, default: field.enumValues.first ?? "")) {
                ForEach(field.enumValues, id: \.self) { option in
                    Text(option).tag(option)
                }
            }
            .labelsHidden()
            .accessibilityLabel(Text(field.title))
        case "integer", "number":
            TextField(field.title, text: binding(for: field.id, default: ""))
                .textFieldStyle(.roundedBorder)
                .accessibilityLabel(Text(field.title))
        case "array":
            TextField(
                field.title,
                text: binding(for: field.id, default: "")
            )
            .textFieldStyle(.roundedBorder)
            .accessibilityLabel(Text(field.title))
            .help("Enter values separated by commas or new lines.")
        default:
            TextField(field.title, text: binding(for: field.id, default: ""))
                .textFieldStyle(.roundedBorder)
                .accessibilityLabel(Text(field.title))
        }
    }

    private func binding(for key: String, default defaultValue: String) -> Binding<String> {
        Binding(
            get: { values[key, default: defaultValue] },
            set: { values[key] = $0 }
        )
    }

    private func binding(for key: String, default defaultValue: Bool) -> Binding<Bool> {
        Binding(
            get: { booleans[key, default: defaultValue] },
            set: { booleans[key] = $0 }
        )
    }
}

struct UnsupportedElicitationSchemaCard: View {
    let message: String
    let onDecline: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label("Unsupported form schema", systemImage: "exclamationmark.triangle")
                .font(.headline)
            Text(message)
            Text("This form cannot be rendered safely. Ask the server for URL mode or simplify the schema.")
                .font(.caption)
                .foregroundStyle(.secondary)
            HStack {
                Button("Decline", action: onDecline)
                Button("Cancel", role: .cancel, action: onCancel)
            }
        }
        .padding(10)
        .background(.orange.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
    }
}

struct SecretElicitationBlockedCard: View {
    let onDecline: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label("Use trusted browser flow", systemImage: "lock.shield")
                .font(.headline)
            Text("This form asks for secret-looking values. tamtri does not collect passwords, tokens, or API keys in form mode.")
                .font(.body)
            HStack {
                Button("Decline", action: onDecline)
                Button("Cancel", role: .cancel, action: onCancel)
            }
        }
        .padding(10)
        .background(.red.opacity(0.12), in: RoundedRectangle(cornerRadius: 8))
    }
}
