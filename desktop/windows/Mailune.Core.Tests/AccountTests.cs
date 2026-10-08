using Mailune.Core;

namespace Mailune.Core.Tests;

[TestClass]
public sealed class AccountTests
{
    [TestMethod]
    public void AnAccountRoundTripsThroughJson()
    {
        var account = new Account("ada", "imap.example");
        Assert.AreEqual(account, Account.RoundTrip(account));
    }
}
