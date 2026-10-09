import MailuneModel
import SwiftUI

/// Address and name in, then autoconfig and sign-in, then the account is saved.
public struct OnboardingView: View {
    private let store: any PreferencesStore
    private let vault: any SecretVault
    private let services: OnboardingServices
    @State private var address = ""
    @State private var name = ""
    @State private var working = false
    @State private var failure: OnboardingError?
    @State private var added: AccountSetting?
    @Environment(\.dismiss) private var dismiss

    public init(store: any PreferencesStore, vault: any SecretVault, services: OnboardingServices = .stub) {
        self.store = store
        self.vault = vault
        self.services = services
    }

    public var body: some View {
        Form {
            TextField(Copy.text("onboarding.address"), text: $address)
                .textContentType(.emailAddress)
            TextField(Copy.text("onboarding.name"), text: $name)
            if let failure {
                Text(Self.message(for: failure, address: address))
                    .foregroundStyle(.red)
            }
            if let added, let incoming = added.incoming, let outgoing = added.outgoing {
                Text(Copy.format("onboarding.added", added.address))
                ForEach([incoming, outgoing], id: \.host) { server in
                    Text(verbatim: "\(server.host):\(server.port) · \(server.security.rawValue.uppercased())")
                }
            }
        }
        .formStyle(.grouped)
        .font(MailuneType.body)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button(Copy.text(added == nil ? "sheets.cancel" : "onboarding.done")) { dismiss() }
            }
            ToolbarItem(placement: .confirmationAction) {
                Button(Copy.text("auth.sign_in")) { Task { await signIn() } }
                    .disabled(working || added != nil || address.allSatisfy(\.isWhitespace))
            }
        }
    }

    private func signIn() async {
        working = true
        defer { working = false }
        do {
            added = try await Onboarding.addAccount(
                address: address, displayName: name, services: services, vault: vault, store: store
            )
            failure = nil
        } catch let error as OnboardingError {
            failure = error
        } catch {
            failure = .noConfiguration
        }
    }

    nonisolated static func message(for error: OnboardingError, address: String) -> String {
        switch error {
        case .notAnAddress: Copy.text("onboarding.not_an_address")
        case .alreadyAdded: Copy.format("onboarding.already_added", address)
        case .noConfiguration: Copy.format("onboarding.no_configuration", address)
        case .insecureOnly: Copy.text("onboarding.insecure_only")
        case .stateMismatch, .emptyCode: Copy.text("onboarding.sign_in_failed")
        }
    }
}
