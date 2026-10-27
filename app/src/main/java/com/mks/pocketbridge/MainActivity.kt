package com.mks.pocketbridge

import android.Manifest
import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.Switch
import android.widget.TextView
import android.widget.Toast

class MainActivity : Activity() {
    private lateinit var sw: Switch
    private lateinit var status: TextView
    private lateinit var info: TextView
    private lateinit var etPass: EditText
    private lateinit var etUpstream: EditText
    private lateinit var etKey: EditText
    private var rendering = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        Prefs.init(this)

        sw = findViewById(R.id.sw)
        status = findViewById(R.id.status)
        info = findViewById(R.id.info)
        etPass = findViewById(R.id.etPass)
        etUpstream = findViewById(R.id.etUpstream)
        etKey = findViewById(R.id.etKey)

        sw.setOnCheckedChangeListener { _, on ->
            if (rendering) return@setOnCheckedChangeListener
            val i = Intent(this, BridgeService::class.java)
            if (on) { save(); startForegroundService(i) } else stopService(i)
        }
        findViewById<Button>(R.id.btnSave).setOnClickListener {
            save()
            Toast.makeText(this, "Tersimpan", Toast.LENGTH_SHORT).show()
            render()
        }
        info.setOnLongClickListener {
            val ip = NetUtil.lanAddresses().firstOrNull()?.second ?: return@setOnLongClickListener false
            val cm = getSystemService(ClipboardManager::class.java)
            cm.setPrimaryClip(ClipData.newPlainText("url", "http://$ip:${Native.WEB_PORT}"))
            Toast.makeText(this, "URL disalin", Toast.LENGTH_SHORT).show()
            true
        }

        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1)
        }
    }

    // Re-read the addresses while visible so switching WiFi / hotspot updates the screen.
    private val ticker = object : Runnable {
        override fun run() {
            render()
            info.postDelayed(this, 3000)
        }
    }

    override fun onResume() {
        super.onResume()
        // Settings can also be changed from the web UI, so reload them every time the screen shows.
        etPass.setText(Prefs.password)
        etUpstream.setText(Prefs.upstream)
        etKey.setText(Prefs.apiKey)
        BridgeState.listener = { render() }
        ticker.run()
    }

    override fun onPause() {
        BridgeState.listener = null
        info.removeCallbacks(ticker)
        super.onPause()
    }

    private fun save() =
        Prefs.save(etPass.text.toString(), etUpstream.text.toString().trim().trimEnd('/'), etKey.text.toString().trim())

    private fun render() {
        rendering = true
        sw.isChecked = BridgeState.running
        rendering = false
        status.text = if (BridgeState.running) "● Aktif" else "○ Mati"
        status.setTextColor(getColor(if (BridgeState.running) R.color.ok else R.color.muted))

        if (!BridgeState.running) {
            info.text = "Nyalakan switch, lalu buka alamatnya di laptop yang terhubung ke WiFi/hotspot yang sama."
            return
        }
        val addrs = NetUtil.lanAddresses()
        val sb = StringBuilder()
        if (addrs.isEmpty()) {
            sb.append("Belum ada jaringan lokal.\nNyalakan hotspot atau sambung ke WiFi.")
        } else {
            sb.append("Buka di laptop:\n")
            addrs.forEach { (_, ip) -> sb.append("http://$ip:${Native.WEB_PORT}\n") }
            sb.append("\nTekan lama untuk menyalin.")
        }
        BridgeState.error?.let { sb.append("\n\n⚠ ").append(it) }
        info.text = sb.toString()
    }
}
