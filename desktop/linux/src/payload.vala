// Decoder for the C JSON records in payloads.schema.json.
// Meson does not compile this file.
namespace Mailune {
    public class Payload : Object {
        public string? message;
        public string kind;
        public string? value;
        public int year;
        public int month;
        public int day;

        public void read_term(Json.Object term) {
            kind = term.get_string_member("kind");
            if (term.has_member("value")) {
                value = term.get_string_member("value");
            }
            if (term.has_member("year")) {
                year = (int) term.get_int_member("year");
                month = (int) term.get_int_member("month");
                day = (int) term.get_int_member("day");
            }
        }

        public void read_error(Json.Object error) {
            message = error.get_string_member("message");
        }
    }
}
