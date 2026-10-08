namespace Mailune.AppCore;

/// <summary>One row in the thread list. The subject is fixture text.</summary>
public sealed record ThreadRow(string Subject);

/// <summary>The list the shell shows. One fixture row is enough to name the shape.</summary>
public sealed record ThreadListDescription(ThreadRow Row)
{
    public static ThreadListDescription Open() => new(new ThreadRow("Thursday"));
}
