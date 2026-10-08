using System.Text.Json;

namespace Mailune.Core;

/// <summary>
/// One account the shell can name. The host is a server name, never a password.
/// </summary>
public sealed record Account(string Id, string Host)
{
    /// <summary>
    /// Writes the account to JSON and reads it back.
    /// </summary>
    public static Account RoundTrip(Account account)
    {
        var json = JsonSerializer.Serialize(account);
        var parsed = JsonSerializer.Deserialize<Account>(json);
        if (parsed is null)
        {
            throw new InvalidOperationException("account json was empty");
        }

        return parsed;
    }
}
