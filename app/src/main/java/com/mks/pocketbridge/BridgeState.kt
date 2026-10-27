package com.mks.pocketbridge

import android.os.Handler
import android.os.Looper

object BridgeState {
    @Volatile var running = false
    @Volatile var error: String? = null
    var listener: (() -> Unit)? = null

    private val main = Handler(Looper.getMainLooper())

    fun changed() {
        main.post { listener?.invoke() }
    }
}
