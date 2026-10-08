using System.Text.Json;
using Mailune.AppCore;

namespace Mailune.AppCore.Tests;

[TestClass]
public sealed class OnDeviceModelTests
{
    [TestMethod]
    public void APromptReturnsScriptedJson()
    {
        var model = new ScriptedModel("""{"summary":"The build is ready."}""");
        var json = model.Complete("summarize the thread");
        using var document = JsonDocument.Parse(json);
        Assert.AreEqual("The build is ready.", document.RootElement.GetProperty("summary").GetString());
    }
}
