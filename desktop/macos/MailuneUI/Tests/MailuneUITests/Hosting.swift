import SwiftUI
#if os(macOS)
    import AppKit

    typealias PlatformView = NSView
#else
    import UIKit

    typealias PlatformView = UIView
#endif

/// Puts a SwiftUI view in a real window and lays it out, on macOS and iOS
/// alike, so a test sees the platform views SwiftUI actually built.
@MainActor
enum Hosting {
    /// Windows stay alive for the whole run; a released window would take
    /// the hosted view tree with it before the test inspects it.
    private static var windows: [AnyObject] = []

    static func render(_ view: some View, width: CGFloat = 800, height: CGFloat = 600) -> PlatformView {
        let frame = CGRect(x: 0, y: 0, width: width, height: height)
        #if os(macOS)
            let host = NSHostingView(rootView: view)
            host.frame = frame
            let window = NSWindow(contentRect: frame, styleMask: [.titled], backing: .buffered, defer: false)
            window.contentView = host
            windows.append(window)
            host.layoutSubtreeIfNeeded()
            return host
        #else
            let controller = UIHostingController(rootView: view)
            let window = UIWindow(frame: frame)
            window.rootViewController = controller
            window.isHidden = false
            windows.append(window)
            controller.view.layoutIfNeeded()
            return controller.view
        #endif
    }

    static func classNames(in view: PlatformView) -> [String] {
        [String(describing: type(of: view))] + view.subviews.flatMap { classNames(in: $0) }
    }
}
