using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class UiScenarioTests
{
    [TestMethod]
    public void TheScenarioNamesAFixtureSubject()
    {
        Assert.AreEqual("Thursday", UiScenario.Open().Subject);
    }
}
