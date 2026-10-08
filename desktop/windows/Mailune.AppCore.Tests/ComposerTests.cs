using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class ComposerTests
{
    [TestMethod]
    public void SendStaysOffUntilConfirm()
    {
        var draft = ComposerDescription.Draft();
        Assert.AreEqual("Grace", draft.Recipient);
        Assert.AreEqual("Thursday", draft.Subject);
        Assert.AreEqual("Can we meet Thursday afternoon?", draft.Body);
        Assert.IsFalse(draft.CanSend);
        Assert.IsTrue(draft.Confirm().CanSend);
    }
}
