namespace Mailune.AppCore;

/// <summary>A notification the shell asked the host to show. The title is not a secret.</summary>
public sealed record HostNotification(string Title);

/// <summary>
/// Notifications, connectivity, and the OAuth redirect, all in memory.
/// Nothing here opens a socket or a browser.
/// </summary>
public sealed class FakeHost
{
    public List<HostNotification> Notifications { get; } = [];

    public bool Online { get; private set; } = true;

    public string? Redirect { get; private set; }

    public void Notify(string title) => Notifications.Add(new HostNotification(title));

    public void SetOnline(bool online) => Online = online;

    /// <summary>
    /// Records the redirect the host would hand back after sign-in.
    /// The value stays in this process.
    /// </summary>
    public void CompleteOAuth(string code) => Redirect = "mailune:oauth?code=" + code;
}
