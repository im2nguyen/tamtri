import Foundation
import Security
@testable import Tamtri
import XCTest

final class KeychainCredentialStoreTests: XCTestCase {
    func testSaveAndLoadRoundTrip() throws {
        let ref = "tamtri-test-\(UUID().uuidString)"
        defer { delete(ref: ref) }

        try KeychainCredentialStore.save(value: "hello", for: ref)
        XCTAssertEqual(KeychainCredentialStore.load(for: ref), "hello")

        try KeychainCredentialStore.save(value: "goodbye", for: ref)
        XCTAssertEqual(KeychainCredentialStore.load(for: ref), "goodbye")
    }

    func testOAuthTokenStoreRoundTrip() throws {
        let ref = "tamtri-oauth-test-\(UUID().uuidString)"
        defer { delete(ref: ref) }

        try OAuthTokenStore.save(bundleJSON: #"{"access_token":"x"}"#, for: ref)
        XCTAssertEqual(OAuthTokenStore.load(for: ref), #"{"access_token":"x"}"#)
    }

    private func delete(ref: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "tamtri.gateway",
            kSecAttrAccount as String: ref
        ]
        SecItemDelete(query as CFDictionary)
    }
}

