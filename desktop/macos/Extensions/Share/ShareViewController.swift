//! Share extension view. It shows the fixture subject and does not upload mail.

import UIKit

final class ShareViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
        let label = UILabel(frame: view.bounds)
        label.text = ShareMail.subject
        label.accessibilityIdentifier = "share-subject"
        view.addSubview(label)
    }
}
