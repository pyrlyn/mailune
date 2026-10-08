using System.Text.Json;

namespace Mailune.AppCore;

/// <summary>
/// On-device completion. This machine cannot call a Windows AI API, so the
/// only implementation is a script that returns prepared JSON.
/// </summary>
public interface IOnDeviceModel
{
    /// <summary>Returns JSON for <paramref name="prompt"/>. The prompt is not a secret.</summary>
    string Complete(string prompt);
}

/// <summary>Returns one prepared JSON document for any non-empty prompt.</summary>
public sealed class ScriptedModel : IOnDeviceModel
{
    private readonly string json;

    public ScriptedModel(string json) => this.json = json;

    public string Complete(string prompt)
    {
        if (string.IsNullOrWhiteSpace(prompt))
        {
            throw new InvalidOperationException("prompt is empty");
        }

        JsonDocument.Parse(json).Dispose();
        return json;
    }
}
