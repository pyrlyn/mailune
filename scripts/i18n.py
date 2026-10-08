"""Convert the gettext catalogs in i18n/ into each shell's native format.

i18n/mailune.pot is the English source and i18n/<lang>.po holds one translation.
Gettext stays the single source of truth so translators work in one format
whatever the shell; the native files are build output in target/i18n/:

    apple/<lang>.lproj/Localizable.strings and Localizable.stringsdict
    android/values[-<lang>]/strings.xml
    windows/<lang>/Resources.resw
    linux/<lang>/LC_MESSAGES/mailune.mo
    web/<lang>.json (i18next v4)

Every message is keyed by its msgctxt. Placeholders are positional `{0}`, `{1}`;
in a plural message `{0}` is the count. Run it through `mise run i18n`, which
supplies translate-toolkit; `--check` validates the catalogs without writing.
"""

import argparse
import re
import shutil
import sys
from pathlib import Path

from translate.lang import data
from translate.misc.multistring import multistring
from translate.storage import aresource, jsonl10n, mo, pypo, resx, stringsdict
from translate.storage.properties import stringsutf8file

BASE = "en"
DOMAIN = "mailune"
KEY = re.compile(r"[a-z][a-z0-9_]*(\.[a-z0-9_]+)*")
PLACEHOLDER = re.compile(r"\{(\d+)\}")


class Message:
    def __init__(self, key, source, target):
        self.key = key
        self.source = source  # one form, or singular and plural
        self.target = target  # gettext forms of the language; empty when untranslated

    @property
    def plural(self):
        return len(self.source) > 1


def forms(value):
    return [str(s) for s in value.strings] if isinstance(value, multistring) else [str(value)]


def placeholders(text):
    return set(PLACEHOLDER.findall(text))


def stray_braces(text):
    return "{" in PLACEHOLDER.sub("", text) or "}" in PLACEHOLDER.sub("", text)


def flat_key(key):
    # Android resource names must be Java identifiers, and a dot in a .resw name
    # is read as a property path by x:Uid.
    return key.replace(".", "_")


def load(path, errors):
    store = pypo.pofile.parsefile(str(path))
    units = {}
    for unit in store.units:
        if unit.isheader() or unit.isobsolete():
            continue
        key = unit.getcontext()
        where = f"{path.name}: {unit.source!r}"
        if not key:
            errors.append(f"{where}: no msgctxt key")
            continue
        if not KEY.fullmatch(key):
            errors.append(f"{where}: key {key!r} is not dotted snake_case")
        if key in units:
            errors.append(f"{where}: duplicate key {key!r}")
        units[key] = unit
    return store, units


def check_keys(keys, errors):
    flat = {}
    for key in keys:
        other = flat.setdefault(flat_key(key), key)
        if other != key:
            errors.append(f"keys {other!r} and {key!r} collide as {flat_key(key)!r}")
    for key in keys:
        if any(k.startswith(key + ".") for k in keys):
            errors.append(f"key {key!r} is also a group of keys, which nested JSON cannot hold")


def check_message(lang, key, source, target, nplurals, errors):
    where = f"{lang}.po: {key}"
    allowed = set().union(*(placeholders(s) for s in source))
    for text in source + target:
        if stray_braces(text):
            errors.append(f"{where}: braces other than {{N}} in {text!r}")
    if not target:
        return
    if len(source) > 1 and len(target) != nplurals:
        errors.append(f"{where}: {len(target)} plural forms, the language has {nplurals}")
    for text in target:
        found = placeholders(text)
        # A plural form may leave the count out ("one message"); nothing else may differ.
        if found - allowed or (len(source) == 1 and found != allowed):
            errors.append(f"{where}: placeholders {sorted(found)} do not match {sorted(allowed)}")


def plural_forms(store, lang):
    header = store.parseheader().get("Plural-Forms", "") if store else ""
    if "nplurals=" not in header.replace(" ", ""):
        _, n, rule = data.languages.get(lang, (None, 2, "(n != 1)"))
        header = f"nplurals={n}; plural={rule};"
    return header


def nplurals(rule):
    return int(re.search(r"nplurals\s*=\s*(\d+)", rule)[1])


def read_catalogs(src):
    """Return ({lang: ([Message], Plural-Forms)}, errors); the template is the base language."""
    errors = []
    _, template = load(src / f"{DOMAIN}.pot", errors)
    check_keys(list(template), errors)
    base = []
    for key, unit in template.items():
        source = forms(unit.source)
        check_message(BASE, key, source, [], 2, errors)
        base.append(Message(key, source, source))
    catalogs = {BASE: (base, plural_forms(None, BASE))}
    for path in sorted(src.glob("*.po")):
        store, units = load(path, errors)
        lang = store.gettargetlanguage() or path.stem
        rule = plural_forms(store, lang)
        for key in units.keys() - template.keys():
            errors.append(f"{path.name}: key {key!r} is not in {DOMAIN}.pot")
        messages = []
        for key, unit in template.items():
            source = forms(unit.source)
            po = units.get(key)
            target = forms(po.target) if po is not None and po.istranslated() else []
            if po is not None and forms(po.source) != source:
                errors.append(f"{path.name}: {key} has a stale msgid; run msgmerge")
            check_message(lang, key, source, target, nplurals(rule), errors)
            messages.append(Message(key, source, target))
        catalogs[lang] = (messages, rule)
    return catalogs, errors


def printf(text, spec, count_spec):
    """`{0}` → `%1$<spec>`; `%` is doubled only where the string is a format string."""
    if not PLACEHOLDER.search(text):
        return text
    text = text.replace("%", "%%")
    return PLACEHOLDER.sub(
        lambda m: f"%{int(m[1]) + 1}${count_spec if count_spec and m[1] == '0' else spec}",
        text,
    )


def mustache(text, plural):
    return PLACEHOLDER.sub(
        lambda m: "{{count}}" if plural and m[1] == "0" else "{{%s}}" % m[1], text
    )


def by_tag(lang, target, tags):
    """Spread gettext forms over the CLDR tags a native format asks for."""
    mapping = dict(zip(data.get_cldr_plural_tags(lang, len(target)), target))
    # Gettext often merges CLDR categories (French "many" and "other" share a
    # form), so a tag without its own form takes the last, most general one.
    return [mapping.get(tag, mapping.get("other", target[-1])) for tag in tags]


def add(store, key, source, target, **unit_args):
    unit = store.UnitClass(source, **unit_args)
    unit.setid(key)
    store.addunit(unit)
    unit.target = target
    return unit


def apple(lang, messages):
    strings, plurals = stringsutf8file(), stringsdict.StringsDictFile()
    plurals.settargetlanguage(lang)
    for m in (m for m in messages if m.target):
        if not m.plural:
            # A bare unit is a Java .properties line; the store's dialect is not inherited.
            target = printf(m.target[0], "@", None)
            add(strings, m.key, m.source[0], target, personality="strings-utf8")
            continue
        target = multistring([printf(t, "@", "ld") for t in m.target])
        tags = plurals._get_target_plural_tags(target)
        filled = by_tag(lang, [str(t) for t in target.strings], tags)
        if tags[0] == "zero" and "zero" not in data.get_cldr_plural_tags(lang, len(m.target)):
            filled[0] = ""  # stringsdict offers "zero" to every language; leave it unset
        unit = add(plurals, f"{m.key}:count", m.source[0], multistring(filled))
        unit.format_value_type = "ld"
    return strings, plurals


def android(lang, messages):
    store = aresource.AndroidResourceFile()
    store.settargetlanguage(lang)
    tags = store.get_plural_tags()
    for m in (m for m in messages if m.target):
        count = "d" if m.plural else None
        target = [printf(t, "s", count) for t in m.target]
        value = multistring(by_tag(lang, target, tags)) if m.plural else target[0]
        add(store, flat_key(m.key), printf(m.source[0], "s", count), value)
    return store


def windows(lang, messages):
    # .resw has no plurals: each CLDR form becomes its own <key>_<tag> entry.
    store = resx.RESXFile()
    for m in (m for m in messages if m.target):
        if not m.plural:
            add(store, flat_key(m.key), m.source[0], m.target[0])
            continue
        tags = data.get_cldr_plural_tags(lang, len(m.target))
        for tag, text in zip(tags, by_tag(lang, m.target, tags)):
            add(store, f"{flat_key(m.key)}_{tag}", m.source[0], text)
    return store


def web(lang, messages):
    store = jsonl10n.I18NextV4File()
    store.settargetlanguage(lang)
    tags = store.get_plural_tags()
    for m in (m for m in messages if m.target):
        target = [mustache(t, m.plural) for t in m.target]
        value = multistring(by_tag(lang, target, tags)) if m.plural else target[0]
        add(store, m.key, mustache(m.source[0], m.plural), value)
    return store


def linux(messages, rule):
    # msgid is rewritten too: the Vala shell looks strings up by msgctxt and the
    # printf-style English text it passes to C_() / ngettext.
    store = mo.mofile()
    # Without a header gettext reads the catalog as ASCII and ngettext has no rule.
    header = store.UnitClass("")
    store.addunit(header)
    header.target = f"Content-Type: text/plain; charset=UTF-8\nPlural-Forms: {rule}\n"
    for m in (m for m in messages if m.target):
        source = [printf(s, "s", "d" if m.plural else None) for s in m.source]
        target = [printf(t, "s", "d" if m.plural else None) for t in m.target]
        unit = store.UnitClass(multistring(source) if m.plural else source[0])
        unit.setcontext(m.key)
        store.addunit(unit)
        unit.target = multistring(target) if m.plural else target[0]
    return store


def bcp47(lang):
    return lang.replace("_", "-")


def android_dir(lang):
    if lang == BASE:
        return "values"
    parts = lang.replace("-", "_").split("_")
    if len(parts) == 2 and len(parts[1]) == 2:
        return f"values-{parts[0]}-r{parts[1].upper()}"
    return "values-" + parts[0] if len(parts) == 1 else "values-b+" + "+".join(parts)


def write(path, store):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(bytes(store))


def build(catalogs, out):
    if out.exists():
        shutil.rmtree(out)
    for lang, (messages, rule) in catalogs.items():
        strings, plurals = apple(lang, messages)
        lproj = out / "apple" / f"{bcp47(lang)}.lproj"
        write(lproj / "Localizable.strings", strings)
        write(lproj / "Localizable.stringsdict", plurals)
        write(out / "android" / android_dir(lang) / "strings.xml", android(lang, messages))
        write(out / "windows" / bcp47(lang) / "Resources.resw", windows(lang, messages))
        write(out / "web" / f"{bcp47(lang)}.json", web(lang, messages))
        if lang != BASE:  # the English text is the msgid itself
            write(out / "linux" / lang / "LC_MESSAGES" / f"{DOMAIN}.mo", linux(messages, rule))


def main(argv=None):
    root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="validate only, write nothing")
    parser.add_argument("--src", type=Path, default=root / "i18n")
    parser.add_argument("--out", type=Path, default=root / "target" / "i18n")
    args = parser.parse_args(argv)
    catalogs, errors = read_catalogs(args.src)
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1
    if not args.check:
        build(catalogs, args.out)
    for lang, (messages, _) in catalogs.items():
        done = sum(1 for m in messages if m.target)
        print(f"{lang}: {done}/{len(messages)} translated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
