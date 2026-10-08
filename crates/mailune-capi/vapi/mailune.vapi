/*
 * mailune-capi for Vala. Each call returns one JSON record.
 * The string is freed with mailune_string_free, not g_free.
 */
[CCode (cheader_filename = "mailune.h")]
namespace Mailune {
    [CCode (cname = "mailune_ready", free_function = "mailune_string_free")]
    public string ready ();

    [CCode (cname = "mailune_parse_query", free_function = "mailune_string_free")]
    public string parse_query (string query);
}
