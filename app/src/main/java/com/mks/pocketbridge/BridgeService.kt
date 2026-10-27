package com.mks.pocketbridge

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.drawable.Icon
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager

class BridgeService : Service() {
    companion object {
        const val ACTION_STOP = "com.mks.pocketbridge.STOP"
        private const val CHANNEL = "bridge"
        private const val NOTIF_ID = 1
    }

    private var started = false
    private var wakeLock: PowerManager.WakeLock? = null
    private var wifiLock: WifiManager.WifiLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Prefs.init(this)
        val n = buildNotification()
        if (Build.VERSION.SDK_INT >= 34) startForeground(NOTIF_ID, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        else startForeground(NOTIF_ID, n)

        if (intent?.action == ACTION_STOP) {
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
            return START_NOT_STICKY
        }
        if (!started) startServers()
        return START_STICKY
    }

    private fun startServers() {
        val errors = Native.start(filesDir.absolutePath)
        started = true

        wakeLock = (getSystemService(POWER_SERVICE) as PowerManager)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "PocketBridge:server").apply { acquire() }
        @Suppress("DEPRECATION")
        wifiLock = (applicationContext.getSystemService(WIFI_SERVICE) as WifiManager)
            .createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, "PocketBridge:wifi").apply { acquire() }

        BridgeState.running = true
        BridgeState.error = errors.ifEmpty { null }
        BridgeState.changed()
    }

    override fun onDestroy() {
        if (started) Native.stop()
        started = false
        wakeLock?.let { if (it.isHeld) it.release() }
        wifiLock?.let { if (it.isHeld) it.release() }
        BridgeState.running = false
        BridgeState.error = null
        BridgeState.changed()
        super.onDestroy()
    }

    private fun buildNotification(): Notification {
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel(CHANNEL, "PocketBridge", NotificationManager.IMPORTANCE_LOW))
        val ip = NetUtil.lanAddresses().firstOrNull()?.second
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val stop = PendingIntent.getService(
            this, 1, Intent(this, BridgeService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE,
        )
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(R.drawable.ic_stat)
            .setContentTitle("PocketBridge aktif")
            .setContentText(if (ip != null) "http://$ip:${Native.WEB_PORT}" else "Menunggu WiFi/hotspot…")
            .setOngoing(true)
            .setContentIntent(open)
            .addAction(Notification.Action.Builder(Icon.createWithResource(this, R.drawable.ic_stat), "Matikan", stop).build())
            .build()
    }
}
