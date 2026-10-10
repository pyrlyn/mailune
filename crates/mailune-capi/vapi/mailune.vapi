/*
 * mailune-capi for Vala: include/mailune.h, bound by hand.
 *
 * Every call answers with one JSON string, {"ok": <value>} or
 * {"error": {"type": ..., "message": ...}}; parse it with json-glib.
 * schema/payloads.schema.json describes each value, and the comment on each
 * function in include/mailune.h names which one `ok` holds. The answer is
 * malloc'ed, so Vala's g_free is the right way to release it.
 */
[CCode (cheader_filename = "mailune.h")]
namespace Mailune {
    [CCode (cname = "mailune_version")]
    public string version ();

    /* The view models one window renders. Safe to use from several threads
     * at once. */
    [Compact]
    [CCode (cname = "MailuneCore", free_function = "mailune_core_free")]
    public class Core {
        [CCode (cname = "mailune_core_new")]
        public Core ();

        /* `events` is a JSON array of `Event`; `ok` is a `ViewState`. */
        [CCode (cname = "mailune_core_fold")]
        public string fold (string events);

        /* `ok` is a `ViewState`. */
        [CCode (cname = "mailune_core_state")]
        public string state ();

        /* `msg` is one JSON `UiMsg` for the shared reducer; `ok` is a
         * `UiState`. */
        [CCode (cname = "mailune_core_dispatch")]
        public string dispatch (string msg);
    }
}
