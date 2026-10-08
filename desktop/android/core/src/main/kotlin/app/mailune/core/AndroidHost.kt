package app.mailune.core

/**
 * In-process stand-ins for secrets, sync, notifications, and the OAuth redirect.
 * Nothing here opens a socket or a keychain.
 */
class AndroidHost {
    private val secrets = linkedMapOf<String, String>()
    private val syncs = mutableListOf<String>()

    /** Stores a secret for [account]. */
    fun putSecret(account: String, secret: String) {
        if (account.isEmpty() || secret.isEmpty()) {
            throw IllegalArgumentException("empty secret")
        }
        secrets[account] = secret
    }

    /** The secret stored for [account], if one was put. */
    fun secret(account: String): String? = secrets[account]

    /** Records that [account] should sync. */
    fun requestSync(account: String) {
        if (account.isEmpty()) {
            throw IllegalArgumentException("empty account")
        }
        syncs.add(account)
    }

    /** Accounts that asked to sync, in order. */
    fun syncRequests(): List<String> = syncs.toList()

    /** The notification channel id the fake would post on. */
    fun notificationChannel(): String = "mailune.sync"

    /** Redirect the fake returns after a code arrives. */
    fun oauthRedirect(code: String): String {
        if (code.isEmpty()) {
            throw IllegalArgumentException("empty code")
        }
        return "mailune://callback?code=$code"
    }

    override fun toString(): String = "AndroidHost"
}
