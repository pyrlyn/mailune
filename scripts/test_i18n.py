"""Tests for scripts/i18n.py over small catalogs with plurals (`mise run i18n:test`)."""

import contextlib
import gettext
import io
import json
import plistlib
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import i18n  # noqa: E402

POT = """msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\\n"

msgctxt "list.unread"
msgid "{0} unread"
msgid_plural "{0} unread"
msgstr[0] ""
msgstr[1] ""

msgctxt "thread.from"
msgid "{0} · from {1} (100%)"
msgstr ""

msgctxt "folder.inbox"
msgid "Inbox"
msgstr ""
"""


def po(lang, plural_forms, unread, from_, inbox):
    plurals = "".join(f'msgstr[{i}] "{text}"\n' for i, text in enumerate(unread))
    return f"""msgid ""
msgstr ""
"Language: {lang}\\n"
"Content-Type: text/plain; charset=UTF-8\\n"
"Plural-Forms: {plural_forms}\\n"

msgctxt "list.unread"
msgid "{{0}} unread"
msgid_plural "{{0}} unread"
{plurals}
msgctxt "thread.from"
msgid "{{0}} · from {{1}} (100%)"
msgstr "{from_}"

msgctxt "folder.inbox"
msgid "Inbox"
msgstr "{inbox}"
"""


CATALOGS = {
    "mailune.pot": POT,
    "de.po": po(
        "de",
        "nplurals=2; plural=(n != 1);",
        ["{0} ungelesen", "{0} ungelesene"],
        "{0} · von {1} (100%)",
        "Posteingang",
    ),
    "fr.po": po(
        "fr",
        "nplurals=2; plural=(n > 1);",
        ["{0} non lu", "{0} non lus"],
        "{0} · de {1} (100 %)",
        "Boîte de réception",
    ),
    "ru.po": po(
        "ru",
        "nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && "
        "(n%100<10 || n%100>=20) ? 1 : 2);",
        ["{0} непрочитанное", "{0} непрочитанных", "{0} непрочитанных!"],
        "{1} → {0} (100%)",
        "Входящие",
    ),
    "ja.po": po("ja", "nplurals=1; plural=0;", ["未読 {0} 件"], "{1} から {0}", "受信トレイ"),
}


class Converter(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.src, self.out = Path(tmp.name, "i18n"), Path(tmp.name, "out")
        self.src.mkdir()
        for name, text in CATALOGS.items():
            (self.src / name).write_text(text, encoding="utf-8")

    def run_main(self, *extra):
        quiet = io.StringIO()
        with contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet):
            return i18n.main(["--src", str(self.src), "--out", str(self.out), *extra])

    def errors(self):
        return i18n.read_catalogs(self.src)[1]

    def test_check_writes_nothing(self):
        self.assertEqual(self.run_main("--check"), 0)
        self.assertFalse(self.out.exists())

    def test_apple(self):
        self.assertEqual(self.run_main(), 0)
        lproj = self.out / "apple" / "de.lproj"
        strings = (lproj / "Localizable.strings").read_text(encoding="utf-8")
        self.assertIn('"thread.from" = "%1$@ · von %2$@ (100%%)";', strings)
        self.assertIn('"folder.inbox" = "Posteingang";', strings)
        self.assertNotIn("list.unread", strings)
        entry = plistlib.loads((lproj / "Localizable.stringsdict").read_bytes())["list.unread"]
        self.assertEqual(entry["NSStringLocalizedFormatKey"], "%#@count@")
        rule = entry["count"]
        self.assertEqual(rule["NSStringFormatValueTypeKey"], "ld")
        self.assertEqual(rule["one"], "%1$ld ungelesen")
        self.assertEqual(rule["other"], "%1$ld ungelesene")
        self.assertNotIn("zero", rule)
        ja = self.out / "apple" / "ja.lproj" / "Localizable.stringsdict"
        self.assertEqual(plistlib.loads(ja.read_bytes())["list.unread"]["count"]["other"], "未読 %1$ld 件")

    def test_android(self):
        self.assertEqual(self.run_main(), 0)
        ru = ET.parse(self.out / "android" / "values-ru" / "strings.xml").getroot()
        items = {i.get("quantity"): i.text for i in ru.find("plurals[@name='list_unread']")}
        self.assertEqual(items["one"], "%1$d непрочитанное")
        self.assertEqual(items["few"], "%1$d непрочитанных")
        self.assertEqual(items["many"], "%1$d непрочитанных!")
        self.assertEqual(ru.find("string[@name='thread_from']").text, "%2$s → %1$s (100%%)")
        base = ET.parse(self.out / "android" / "values" / "strings.xml").getroot()
        self.assertEqual(base.find("string[@name='folder_inbox']").text, "Inbox")

    def test_windows(self):
        self.assertEqual(self.run_main(), 0)
        root = ET.parse(self.out / "windows" / "fr" / "Resources.resw").getroot()
        values = {d.get("name"): d.find("value").text for d in root.iter("data")}
        self.assertEqual(values["thread_from"], "{0} · de {1} (100 %)")
        self.assertEqual(values["list_unread_one"], "{0} non lu")
        self.assertEqual(values["list_unread_many"], "{0} non lus")
        self.assertEqual(values["list_unread_other"], "{0} non lus")

    def test_web(self):
        self.assertEqual(self.run_main(), 0)
        fr = json.loads((self.out / "web" / "fr.json").read_text(encoding="utf-8"))
        self.assertEqual(fr["list"]["unread_one"], "{{count}} non lu")
        self.assertEqual(fr["list"]["unread_other"], "{{count}} non lus")
        self.assertEqual(fr["thread"]["from"], "{{0}} · de {{1}} (100 %)")
        self.assertEqual(fr["folder"]["inbox"], "Boîte de réception")

    def test_linux(self):
        self.assertEqual(self.run_main(), 0)
        self.assertFalse((self.out / "linux" / "en").exists())
        with open(self.out / "linux" / "ru" / "LC_MESSAGES" / "mailune.mo", "rb") as fp:
            ru = gettext.GNUTranslations(fp)
        unread = ("list.unread", "%1$d unread", "%1$d unread")
        self.assertEqual(ru.npgettext(*unread, 1), "%1$d непрочитанное")
        self.assertEqual(ru.npgettext(*unread, 3), "%1$d непрочитанных")
        self.assertEqual(ru.npgettext(*unread, 5), "%1$d непрочитанных!")
        self.assertEqual(ru.pgettext("folder.inbox", "Inbox"), "Входящие")

    def test_untranslated_is_left_to_the_fallback(self):
        text = CATALOGS["de.po"].replace('msgstr "Posteingang"', 'msgstr ""')
        (self.src / "de.po").write_text(text, encoding="utf-8")
        self.assertEqual(self.run_main(), 0)
        strings = (self.out / "apple" / "de.lproj" / "Localizable.strings").read_text(encoding="utf-8")
        self.assertNotIn("folder.inbox", strings)

    def test_rejects_bad_catalogs(self):
        cases = {
            "no msgctxt key": ('msgctxt "folder.inbox"\n', ""),
            "do not match": ("{0} · von {1} (100%)", "{0} · von (100%)"),
            "braces other than": ('msgstr "Posteingang"', 'msgstr "Post{eingang"'),
            "plural forms": ('msgstr[1] "{0} ungelesene"\n', ""),
        }
        for error, (old, new) in cases.items():
            with self.subTest(error):
                (self.src / "de.po").write_text(CATALOGS["de.po"].replace(old, new, 1), encoding="utf-8")
                self.assertTrue(any(error in e for e in self.errors()), self.errors())
                self.assertEqual(self.run_main(), 1)

    def test_rejects_keys_that_cannot_map(self):
        cases = {
            "is also a group of keys": 'msgctxt "list"\nmsgid "List"\nmsgstr ""\n',
            "collide as": 'msgctxt "list_unread"\nmsgid "Unread"\nmsgstr ""\n',
            "not dotted snake_case": 'msgctxt "List.Unread"\nmsgid "Unread"\nmsgstr ""\n',
            "not in mailune.pot": None,
        }
        for error, unit in cases.items():
            with self.subTest(error):
                if unit:
                    (self.src / "mailune.pot").write_text(POT + "\n" + unit, encoding="utf-8")
                else:
                    (self.src / "mailune.pot").write_text(POT, encoding="utf-8")
                    extra = '\nmsgctxt "gone"\nmsgid "Gone"\nmsgstr "Weg"\n'
                    (self.src / "de.po").write_text(CATALOGS["de.po"] + extra, encoding="utf-8")
                self.assertTrue(any(error in e for e in self.errors()), self.errors())


if __name__ == "__main__":
    unittest.main()
