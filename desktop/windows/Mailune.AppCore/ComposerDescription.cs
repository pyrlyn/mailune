namespace Mailune.AppCore;

/// <summary>
/// A draft. Send stays off until the person confirms.
/// </summary>
public sealed record ComposerDescription(string Recipient, string Subject, string Body, bool Confirmed)
{
    public bool CanSend => Confirmed;

    public ComposerDescription Confirm() => this with { Confirmed = true };

    public static ComposerDescription Draft() =>
        new("Grace", "Thursday", "Can we meet Thursday afternoon?", false);
}
