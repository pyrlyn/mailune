/*
 * Drives mailune-capi through vapi/mailune.vapi the way the GTK shell will:
 * fold contract JSON, read the envelope with json-glib, check the state.
 */
Json.Object ok_of (string answer) {
    var parser = new Json.Parser ();
    try {
        parser.load_from_data (answer);
    } catch (Error e) {
        error ("answer is not JSON: %s", e.message);
    }
    var root = parser.get_root ().get_object ();
    if (!root.has_member ("ok")) {
        error ("expected ok, got %s", answer);
    }
    return root.get_member ("ok").get_object ();
}

int main (string[] args) {
    Test.init (ref args);

    Test.add_func ("/capi/version", () => {
        assert (Mailune.version ().contains ("\"ok\""));
    });

    Test.add_func ("/capi/fold", () => {
        var core = new Mailune.Core ();
        var events = """[
            {"snapshot": {"threads": [{
                "id": "t1", "account": "local",
                "from": {"name": null, "email": "ada@example.com"},
                "subject": "Hello", "snippet": "plain", "stamp": "t0",
                "message_count": 1, "unread": true, "flagged": false,
                "important": false, "pinned": false, "snoozed": false,
                "draft": false, "has_attachment": false,
                "category": "primary", "mailbox": "inbox", "labels": []
            }]}},
            {"notice": {"message": "open t1"}}
        ]""";
        var state = ok_of (core.fold (events));
        assert (state.get_object_member ("open").get_string_member ("subject") == "Hello");
        assert (ok_of (core.state ()).get_array_member ("list").get_length () == 1);
    });

    Test.add_func ("/capi/dispatch", () => {
        var core = new Mailune.Core ();
        core.dispatch ("\"compose_new\"");
        core.dispatch ("""{"edit_draft": {"draft": {"to": "ada@example.com", "subject": "Hi", "body": "Hello"}}}""");
        assert (ok_of (core.dispatch ("\"send\"")).get_string_member ("pending") == "send");
    });

    return Test.run ();
}
