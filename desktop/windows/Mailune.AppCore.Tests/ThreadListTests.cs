using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class ThreadListTests
{
    [TestMethod]
    public void TheListHasOneFixtureSubject()
    {
        var list = ThreadListDescription.Open();
        Assert.AreEqual("Thursday", list.Row.Subject);
    }
}
