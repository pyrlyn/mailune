using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class ShellStateTests
{
    [TestMethod]
    public void TheShellOpensOnTheSampleAccount()
    {
        var state = ShellState.Open();
        Assert.AreEqual("Mailune", state.Title);
        Assert.AreEqual("ada", state.Account.Id);
        Assert.AreEqual("imap.example", state.Account.Host);
    }
}
