#if os(macOS)
    import AppKit
    import CoreSpotlight
    import UniformTypeIdentifiers

    // AppKit only: iOS has no Dock and presents sharing from a view controller.
    extension HostServices {
        public static func live() -> HostServices {
            HostServices(dock: AppDockBadge(), sharing: AppSharing(), spotlight: AppSpotlight())
        }
    }

    @MainActor
    final class AppDockBadge: DockBadge {
        var label: String? {
            get { NSApp.dockTile.badgeLabel }
            set { NSApp.dockTile.badgeLabel = newValue }
        }
    }

    @MainActor
    final class AppSharing: Sharing {
        func share(_ text: String) {
            guard let view = NSApp.keyWindow?.contentView else { return }
            NSSharingServicePicker(items: [text]).show(relativeTo: .zero, of: view, preferredEdge: .minY)
        }
    }

    @MainActor
    final class AppSpotlight: SpotlightIndex {
        private let domain = "app.mailune.threads"

        func replace(with items: [SpotlightItem]) {
            let domain = domain
            // The handler runs off the main actor, so it rebuilds the Core
            // Spotlight objects from Sendable values instead of capturing them.
            CSSearchableIndex.default().deleteSearchableItems(withDomainIdentifiers: [domain]) { _ in
                CSSearchableIndex.default().indexSearchableItems(Self.searchable(items, domain: domain))
            }
        }

        nonisolated private static func searchable(_ items: [SpotlightItem], domain: String) -> [CSSearchableItem] {
            items.map { item in
                let attributes = CSSearchableItemAttributeSet(contentType: .emailMessage)
                attributes.title = item.title
                attributes.authorNames = [item.sender]
                attributes.textContent = item.text.isEmpty ? nil : item.text
                return CSSearchableItem(uniqueIdentifier: item.id, domainIdentifier: domain, attributeSet: attributes)
            }
        }
    }
#endif
