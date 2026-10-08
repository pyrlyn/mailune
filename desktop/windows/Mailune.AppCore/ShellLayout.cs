namespace Mailune.AppCore;

/// <summary>
/// The three panes and the one accelerator the window advertises.
/// Markup is not compiled on this machine.
/// </summary>
public sealed record ShellLayout(IReadOnlyList<string> Panes, string Accelerator)
{
    public static ShellLayout Open() => new(["Folders", "Threads", "Reading"], "Ctrl+1");
}
