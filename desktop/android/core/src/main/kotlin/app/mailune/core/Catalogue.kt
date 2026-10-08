package app.mailune.core

/**
 * String catalogues. English is always loaded. A key missing from the selected
 * language uses the English line.
 */
class Catalogue(
    private val values: Map<String, String>,
    private val english: Map<String, String>,
) {
    /** The line for [key], falling back to English. */
    fun text(key: String): String {
        return values[key] ?: english[key] ?: throw IllegalArgumentException("missing catalogue key")
    }
}

/** Loads `en` and [language]. [language] is two lowercase letters. */
fun loadCatalogue(language: String): Catalogue {
    if (language.length != 2 || language.any { !it.isLowerCase() }) {
        throw IllegalArgumentException("language must be two lowercase letters")
    }
    val english = parseCatalogue(readCatalogue("en"))
    val selected = if (language == "en") english else parseCatalogue(readCatalogue(language))
    return Catalogue(selected, english)
}

private fun readCatalogue(language: String): String {
    val stream = Catalogue::class.java.getResourceAsStream("/catalogues/$language.txt")
        ?: throw IllegalArgumentException("missing catalogue")
    return stream.bufferedReader().use { it.readText() }
}

private fun parseCatalogue(text: String): Map<String, String> {
    val values = linkedMapOf<String, String>()
    for (line in text.lines()) {
        val trimmed = line.trim()
        if (trimmed.isEmpty() || trimmed.startsWith("#")) {
            continue
        }
        val split = trimmed.indexOf('=')
        if (split <= 0) {
            continue
        }
        val key = trimmed.substring(0, split)
        val value = trimmed.substring(split + 1)
        if (value.isNotEmpty()) {
            values[key] = value
        }
    }
    return values
}
