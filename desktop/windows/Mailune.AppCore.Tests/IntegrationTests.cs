using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class IntegrationTests
{
    [TestMethod]
    public void TheIntegrationNamesAToastActionAndABadgeCount()
    {
        var integration = IntegrationDescription.Open();
        Assert.AreEqual("open-thread", integration.ToastAction);
        Assert.AreEqual(1, integration.BadgeCount);
    }
}
