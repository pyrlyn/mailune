//! Widget that shows the fixture subject. It does not load remote content.

import SwiftUI
import WidgetKit

struct WidgetEntry: TimelineEntry {
    let date: Date
    let subject: String
}

struct WidgetSource: TimelineProvider {
    func placeholder(in context: Context) -> WidgetEntry {
        WidgetEntry(date: Date(), subject: WidgetMail.subject)
    }

    func getSnapshot(in context: Context, completion: @escaping (WidgetEntry) -> Void) {
        completion(WidgetEntry(date: Date(), subject: WidgetMail.subject))
    }

    func getTimeline(in context: Context, completion: @escaping (Timeline<WidgetEntry>) -> Void) {
        let entry = WidgetEntry(date: Date(), subject: WidgetMail.subject)
        completion(Timeline(entries: [entry], policy: .never))
    }
}

struct MailWidget: Widget {
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: "mailune.inbox", provider: WidgetSource()) { entry in
            Text(entry.subject)
        }
        .configurationDisplayName("Inbox")
        .description("Fixture subject")
    }
}

@main
struct MailWidgetBundle: WidgetBundle {
    var body: some Widget {
        MailWidget()
    }
}
