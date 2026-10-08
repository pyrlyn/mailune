using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class CatalogTests
{
    [TestMethod]
    public void AMissingKeyFallsBackToEnglish()
    {
        Assert.AreEqual("Входящие", Catalog.Text("ru", "Inbox"));
        Assert.AreEqual("Send", Catalog.Text("ru", "Send"));
        Assert.AreEqual("Send", Catalog.Text("en", "Send"));
    }
}
