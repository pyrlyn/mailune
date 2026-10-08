namespace Mailune.AppCore;

/// <summary>
/// The reading pane. Remote content and script stay off; the body is fixture text.
/// </summary>
public sealed record ReaderDescription(string Body, bool RemoteContent, bool JavaScript)
{
    public static ReaderDescription Open() => new("Can we meet Thursday afternoon?", false, false);
}
