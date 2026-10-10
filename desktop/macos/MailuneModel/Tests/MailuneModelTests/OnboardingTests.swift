import CryptoKit
import XCTest
@testable import MailuneModel

private struct ForgedListener: RedirectListener {
    var code = "code"
    var state: String?

    func receive(for session: OAuthSession) async throws -> Redirect {
        Redirect(code: code, state: state ?? session.state)
    }
}

private final class CountingIssuer: TokenIssuer, @unchecked Sendable {
    var verifiers: [String] = []

    func exchange(code: String, verifier: String) async throws -> IssuedToken {
        verifiers.append(verifier)
        return IssuedToken(access: "a", refresh: "r")
    }
}

@MainActor
final class OnboardingTests: XCTestCase {
    private func add(
        _ address: String,
        listener: RedirectListener = StubRedirectListener(),
        store: FakePreferencesStore,
        vault: MemorySecretVault
    ) async throws -> AccountSetting {
        let services = OnboardingServices(autoconfig: FixtureAutoconfig(), listener: listener, issuer: StubTokenIssuer())
        return try await Onboarding.addAccount(
            address: address, displayName: "Bea", services: services, vault: vault, store: store
        )
    }

    func testAutoconfigAndOAuthCreateAnAccountInTheFakeStore() async throws {
        let store = FakePreferencesStore()
        let vault = MemorySecretVault()
        let account = try await add(" bea@acme.example ", store: store, vault: vault)

        XCTAssertEqual(account.address, "bea@acme.example")
        XCTAssertEqual(account.incoming, ServerEndpoint(service: .imap, host: "imap.acme.example", port: 993, security: .tls))
        XCTAssertEqual(account.outgoing?.security, .starttls)
        XCTAssertTrue(vault.holds(account: account.id))

        let saved = try FakePreferencesStore(data: store.data).load()
        XCTAssertEqual(saved.accounts.map(\.address), ["ana@acme.example", "bea@acme.example"])
        let json = String(decoding: try XCTUnwrap(store.data), as: UTF8.self)
        XCTAssertFalse(json.contains("stub-access"), "the token stays out of preferences")
        XCTAssertFalse(json.contains("stub-refresh"))
    }

    func testNothingIsSavedWhenAStepFails() async {
        let store = FakePreferencesStore()
        let vault = MemorySecretVault()
        await assertThrows(.notAnAddress) { try await self.add("bea", store: store, vault: vault) }
        await assertThrows(.alreadyAdded) { try await self.add("ANA@acme.example", store: store, vault: vault) }
        await assertThrows(.noConfiguration) { try await self.add("bea@unknown.example", store: store, vault: vault) }
        await assertThrows(.insecureOnly) { try await self.add("bea@oldmail.example", store: store, vault: vault) }
        await assertThrows(.stateMismatch) {
            try await self.add("bea@acme.example", listener: ForgedListener(state: "forged"), store: store, vault: vault)
        }
        await assertThrows(.emptyCode) {
            try await self.add("bea@acme.example", listener: ForgedListener(code: ""), store: store, vault: vault)
        }
        XCTAssertNil(store.data)
    }

    func testThePKCEChallengeIsS256OfTheVerifierTheIssuerReceives() async throws {
        let session = OAuthSession.start()
        let issuer = CountingIssuer()
        _ = try await session.complete(listener: StubRedirectListener(), issuer: issuer)
        XCTAssertEqual(issuer.verifiers, [session.verifier])
        XCTAssertEqual(session.challenge, OAuthSession.base64url(Data(SHA256.hash(data: Data(session.verifier.utf8)))))
        XCTAssertFalse(session.challenge.contains("="))
        XCTAssertNotEqual(OAuthSession.start().state, session.state)
    }

    func testTokensDoNotPrint() {
        XCTAssertEqual(String(describing: IssuedToken(access: "secret", refresh: "secret")), "IssuedToken(redacted)")
    }

    func testOlderSavedAccountsWithoutServersStillDecode() throws {
        let json = #"{"accounts":[{"id":"a1","address":"ana@acme.example","displayName":"Ana"}],"theme":"system","density":"cozy","notifications":true,"onlyVIPs":false,"conversationView":true,"autoAdvance":false,"undoSendSeconds":10,"signature":"","syncDays":90,"wifiOnly":false}"#
        XCTAssertNil(try FakePreferencesStore(data: Data(json.utf8)).load().accounts[0].incoming)
    }

    private func assertThrows(
        _ expected: OnboardingError,
        file: StaticString = #filePath,
        line: UInt = #line,
        _ body: () async throws -> AccountSetting
    ) async {
        do {
            _ = try await body()
            XCTFail("expected \(expected)", file: file, line: line)
        } catch {
            XCTAssertEqual(error as? OnboardingError, expected, file: file, line: line)
        }
    }
}
