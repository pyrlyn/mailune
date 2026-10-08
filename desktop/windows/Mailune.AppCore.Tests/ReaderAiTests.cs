using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class ReaderAiTests
{
    [TestMethod]
    public void TheReaderIncludesASummaryAndOneReplyChip()
    {
        var reader = ReaderDescription.Open();
        Assert.AreEqual("Grace wants to meet Thursday.", reader.Summary);
        Assert.AreEqual("Afternoon", reader.ReplyChip);
    }
}
