package com.mks.pocketbridge

import android.content.Context

/** Settings live in the Rust core (config.json) so the phone and the web UI edit the same values. */
object Prefs {
    @Volatile private var ready = false

    fun init(ctx: Context) {
        if (ready) return
        val app = ctx.applicationContext
        if (!Native.init(app.filesDir.absolutePath)) {
            // Migrate settings saved by older versions in SharedPreferences.
            val sp = app.getSharedPreferences("pocketbridge", Context.MODE_PRIVATE)
            Native.setConfig(
                sp.getString("password", "") ?: "",
                sp.getString("upstream", "") ?: "",
                sp.getString("api_key", "") ?: "",
            )
        }
        ready = true
    }

    val password: String get() = Native.getPassword()
    val upstream: String get() = Native.getUpstream()
    val apiKey: String get() = Native.getApiKey()

    fun save(password: String, upstream: String, apiKey: String) = Native.setConfig(password, upstream, apiKey)
}
