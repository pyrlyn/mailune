package app.mailune.ui

/** Reads `key value` lines. Repeated keys keep every value, in order. */
fun descriptionEntries(text: String): Map<String, List<String>> {
    val values = linkedMapOf<String, MutableList<String>>()
    for (line in text.lines()) {
        val trimmed = line.trim()
        if (trimmed.isEmpty() || trimmed.startsWith("#")) {
            continue
        }
        val space = trimmed.indexOf(' ')
        if (space <= 0) {
            continue
        }
        val key = trimmed.substring(0, space)
        val value = trimmed.substring(space + 1)
        if (value.isEmpty()) {
            continue
        }
        values.getOrPut(key) { mutableListOf() }.add(value)
    }
    return values
}

/** Loads a description shipped with the UI module. */
fun readDescription(name: String): String {
    val stream = Tokens::class.java.getResourceAsStream("/$name")
        ?: throw IllegalArgumentException("missing description")
    return stream.bufferedReader().use { it.readText() }
}
