using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class ShellLayoutTests
{
    [TestMethod]
    public void TheShellNamesThreePanesAndOneAccelerator()
    {
        var layout = ShellLayout.Open();
        CollectionAssert.AreEqual(new[] { "Folders", "Threads", "Reading" }, layout.Panes.ToArray());
        Assert.AreEqual("Ctrl+1", layout.Accelerator);
    }
}
