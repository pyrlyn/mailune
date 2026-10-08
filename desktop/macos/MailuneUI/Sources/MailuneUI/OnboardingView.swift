import MailuneModel
import SwiftUI

/// Asks for an address, then stores the autoconfigured account.
public struct OnboardingView: View {
    @State private var email = ""
    @State private var created: MailAccount?
    @State private var store = FakeAccountStore()

    public init() {}

    public var body: some View {
        VStack(alignment: .leading, spacing: MailuneSpace.m) {
            Text("Add an account")
                .font(MailuneType.title)
            TextField("Email", text: $email)
                .font(MailuneType.body)
            Button("Create") {
                created = store.create(email: email.trimmingCharacters(in: .whitespaces))
            }
            if let created {
                Text("\(created.email) on \(created.host)")
                    .font(MailuneType.body)
            }
        }
        .padding(MailuneSpace.m)
        .frame(minWidth: 380, minHeight: 200)
    }
}
