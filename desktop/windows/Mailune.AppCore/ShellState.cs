using Mailune.Core;

namespace Mailune.AppCore;

/// <summary>
/// What the window shows before any mailbox is open. No UI types live here.
/// </summary>
public sealed record ShellState(string Title, Account Account)
{
    /// <summary>The sample the window opens with. It holds no secret.</summary>
    public static ShellState Open() => new("Mailune", new Account("ada", "imap.example"));
}
