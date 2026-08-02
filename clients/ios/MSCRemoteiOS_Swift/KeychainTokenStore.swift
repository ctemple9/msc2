import Foundation
import Security

enum KeychainTokenStore {
    private static let service = "com.camerontemple.MSCRemoteiOS"
    private static let defaultHostID = "default"

    enum KeychainError: Error {
        case unexpectedStatus(OSStatus)
        case invalidData
    }

    private static func account(forHostID hostID: String) -> String {
        let trimmed = hostID.trimmingCharacters(in: .whitespacesAndNewlines)
        let safeHostID = trimmed.isEmpty ? defaultHostID : trimmed
        return "host-token.\(safeHostID)"
    }

    static func saveToken(_ token: String, forHostID hostID: String = defaultHostID) throws {
        let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            try? deleteToken(forHostID: hostID)
            return
        }
        let data = Data(trimmed.utf8)

        // Delete existing first to keep behavior consistent.
        try? deleteToken(forHostID: hostID)

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account(forHostID: hostID),
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]

        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.unexpectedStatus(status)
        }
    }

    static func loadToken(forHostID hostID: String = defaultHostID) throws -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account(forHostID: hostID),
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var item: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &item)

        if status == errSecItemNotFound {
            return nil
        }

        guard status == errSecSuccess else {
            throw KeychainError.unexpectedStatus(status)
        }

        guard let data = item as? Data,
              let token = String(data: data, encoding: .utf8) else {
            throw KeychainError.invalidData
        }

        let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            try? deleteToken(forHostID: hostID)
            return nil
        }

        return trimmed
    }

    static func deleteToken(forHostID hostID: String = defaultHostID) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account(forHostID: hostID)
        ]

        let status = SecItemDelete(query as CFDictionary)
        if status == errSecSuccess || status == errSecItemNotFound {
            return
        }
        throw KeychainError.unexpectedStatus(status)
    }
}
