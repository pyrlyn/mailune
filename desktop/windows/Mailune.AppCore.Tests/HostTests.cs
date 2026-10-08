using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class HostTests
{
    [TestMethod]
    public void ANotificationIsRecorded()
    {
        var host = new FakeHost();
        host.Notify("new mail");
        Assert.AreEqual(1, host.Notifications.Count);
        Assert.AreEqual("new mail", host.Notifications[0].Title);
    }

    [TestMethod]
    public void NetworkStatusCanGoOffline()
    {
        var host = new FakeHost();
        Assert.IsTrue(host.Online);
        host.SetOnline(false);
        Assert.IsFalse(host.Online);
    }

    [TestMethod]
    public void AnOAuthRedirectStaysInMemory()
    {
        var host = new FakeHost();
        host.CompleteOAuth("state-ok");
        Assert.AreEqual("mailune:oauth?code=state-ok", host.Redirect);
    }
}
