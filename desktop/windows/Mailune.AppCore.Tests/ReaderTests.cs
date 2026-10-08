using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class ReaderTests
{
    [TestMethod]
    public void TheReaderKeepsRemoteContentAndScriptOff()
    {
        var reader = ReaderDescription.Open();
        Assert.AreEqual("Can we meet Thursday afternoon?", reader.Body);
        Assert.IsFalse(reader.RemoteContent);
        Assert.IsFalse(reader.JavaScript);
    }
}
