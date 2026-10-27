package com.mks.pocketbridge

/** Rust core (rust/ → libpocketbridge.so): web 8080, proxy 8888, AI gateway 8090, settings storage. */
object Native {
    const val WEB_PORT = 8080
    const val PROXY_PORT = 8888
    const val GATEWAY_PORT = 8090

    init {
        System.loadLibrary("pocketbridge")
    }

    /** Loads settings from dataDir/config.json; false when the file does not exist yet. */
    external fun init(dataDir: String): Boolean

    /** Returns "" on success, otherwise one line per server that failed to start. */
    external fun start(dataDir: String): String

    external fun stop()

    /** Saves settings (shared with the web UI, persisted by the Rust core). */
    external fun setConfig(password: String, upstream: String, apiKey: String)

    external fun getPassword(): String
    external fun getUpstream(): String
    external fun getApiKey(): String
}
